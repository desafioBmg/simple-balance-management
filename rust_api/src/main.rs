mod cad_usuario;
mod transacao;

use actix_web::{get, post, App, HttpServer, Responder, HttpResponse, HttpRequest};
use actix_web::web::{Data, Json, Path};
use actix_web::http::header::ContentType;
use actix_web::cookie::Cookie;

use sqlx::{Pool, Postgres};
use sqlx::postgres::PgPoolOptions;
use sqlx::types::Uuid;

use anyhow::Result;

use cad_usuario::CadUsuario;
use crate::transacao::Transacao::{CreditoDebito, Transferencia};
use crate::transacao::{Transacao, Transf};
use crate::transacao::CreDeb;
use chrono::{Local, SecondsFormat};

#[tokio::main]
async fn main() -> Result<()> {

	let pool = PgPoolOptions::new()
		.max_connections(5)
		.connect("postgres://postgres:SuperSecreto@localhost/bd_bmg").await?;

	let row : (i32,) = sqlx::query_as("SELECT 1")
		.fetch_one( &pool ).await?;

	println!("Resultado da Query: {}", row.0);

	let _ = HttpServer::new( move || {
		App::new()
			.app_data( Data::new( pool.clone() ) )
			.service(teste )
			.service( status )
			.service( abrir_conta )
			.service( balanco )
	} ).bind("0.0.0.0:7777")?
		.run()
		.await;

	Ok(())
}

#[get("/")]
async fn teste( _req: HttpRequest ) -> impl Responder {
	"<html><title>Teste Desafio</title><body>Funcionou!</body></html>"
}

#[get("/status")]
async fn status( ) -> impl Responder {
	HttpResponse::Ok()
		.append_header(("Content-Type", "application/json"))
		.body( "{ status : 'ok' }")
}

#[get("/")]
async fn hello() -> impl Responder {
	HttpResponse::Ok().body("Hello world!")
}

#[post("/abrirConta")]
async fn abrir_conta( bd : Data<Pool<Postgres>>, usuario: Json<CadUsuario> ) -> impl Responder {

	println!("Chegou!!! {:?}", usuario.0);

	let ag = usuario.agencia.unwrap_or( 1 );
	let email = if let Some( mail ) = usuario.email.as_ref() {
		String::from( mail.as_str() )
	} else {
		String::new()
	};

	// TODO fazer o tratamento de erro para algum problema de comunicação com o BD
	let conta : (i64, ) = sqlx::query_as("SELECT coalesce( MAX( conta ) + 1, 1) FROM usuario WHERE agencia = $1" )
		.bind( ag)
		.fetch_one( bd.get_ref() ).await
		.expect("Erro ao definiro número da conta");

	// TODO fazer o tratamento de erro para algum problema de comunicação com o BD
	let chave : (Uuid, ) = sqlx::query_as("INSERT INTO usuario (agencia, conta, senha, nome, email) VALUES ( $1, $2, $3, $4, $5 ) RETURNING id")
		.bind(ag )
		.bind( conta.0 )
		.bind( usuario.senha.as_str() )
		.bind( usuario.nome.as_str() )
		.bind( email )
		.fetch_one( bd.get_ref() ).await
		.expect("Erro ao cadastrar o cliente");

	// TODO consultar a chave do cliente e criar uma tabela com o nome da chave para armazenar as transações
	sqlx::query(format!("CREATE TABLE public.\"{}_T\" (
	horario :: NOT NULL,
	conta uuid,
	valor numeric NOT NULL
)", chave.0 ).as_str() )
		.execute(bd.get_ref() ).await
		.expect("Erro ao liberar a estrutura de transações do usuário.");

	let mut resp = HttpResponse::Ok()
		.insert_header(ContentType::json() )
		.body( format!( "{{ \"status\" : \"ok\", \"agencia\" : {}, \"conta_corrente\" : {}, \"nome\" : \"{}\" }}", ag, conta.0, usuario.nome) );

	resp.add_cookie( &Cookie::new("user", chave.0.to_string()));

	resp
}

#[get("/{id}")]
async fn balanco( bd : Data<Pool<Postgres>>, usuario: Path<String> ) -> actix_web::Result<impl Responder> {

	let user = usuario.as_ref().clone();

	println!("Chegou!!! {:?}", user.as_str());

	// TODO fazer o tratamento de erro para algum problema de comunicação com o BD
	let saldos : (i64, ) = sqlx::query_as("SELECT saldo FROM usuario WHERE agencia = $1" )
		.bind( user.as_str())
		.fetch_one( bd.get_ref() ).await
		.expect(format!( "Erro ao consultar o saldo do usuario {}", user.as_str() ).as_str());

	let mut resp = HttpResponse::Ok()
		.insert_header(ContentType::json() )
		.body( format!( "{{ \"status\" : \"ok\", \"saldo\" : {} }}", saldos.0 ) );

	let _ = resp.add_cookie( &Cookie::new("user", user.as_str() ) );

	Ok( resp )
}

#[post("/credito")]
async fn transacoes(bd : Data<Pool<Postgres>>, req : HttpRequest, transacao: Json<Transacao> ) -> impl Responder {

	match transacao.0 {
		CreditoDebito(op ) => {
			if valida_saldo(bd.get_ref().clone(), op.user.as_str(), op.valor ).await {
				(op.user, op.valor);
			}

		},
		Transferencia( op ) => {
			if valida_saldo(bd.get_ref().clone(), op.origem.as_str(), op.valor * -1.0 ).await {
				(op.origem, op.valor);
			}

		}
	}

	// TODO Gerar a mensagem de confirmação da transação;
	HttpResponse::Ok()
}

async fn valida_saldo(bd : Pool<Postgres>, usuario : &str, valor : f64 ) -> bool {

	let saldo : (f64, ) = sqlx::query_as("SELECT saldo FROM usuario WHERE id = $1" )
		.bind( usuario)
		.fetch_one( &bd ).await
		.expect("Erro ao definiro número da conta");

	if valor > 0.0 || ( saldo.0 + valor ) >= 0f64 {
		return true;
	}

	false
}

async fn atualiza_saldo(bd : &Pool<Postgres>, op : &CreDeb ) {
	sqlx::query("UPDATE usuario SET  saldo = (saldo + $1) WHERE id = $2" )
		.bind( op.valor)
		.bind( op.user.as_str() )
		.execute( &*bd ).await
		.expect( "Erro ao atualizar o saldo da transação." );
}

async fn transacao_basica(bd : &Pool<Postgres>, op : CreDeb ) {
	atualiza_saldo( &bd, &op).await;

	sqlx::query("INSERT INTO $1 ( horario, valor ) values ( $2, $3 )" )
		.bind( op.valor)
		.bind( op.user.as_str() )
		.execute( &*bd ).await
		.expect( "Erro ao atualizar o saldo da transação." );
}

async fn transacao_transferencia( bd : &Pool<Postgres>, tempo : &str, conta1 : &str, conta2 : &str, valor : f64 ) {
	let mut tabela = format!("\"{}_T\"", conta1);

	sqlx::query("INSERT INTO $1 ( horario, valor, conta ) values ( $2, $3, $4 )" )
		.bind( tabela.as_str() )
		.bind( tempo )
		.bind( valor )
		.bind( conta2 )
		.execute( &*bd ).await
		.expect( "Erro ao atualizar o saldo da transação." );
}

async fn transferencia(   bd : Pool<Postgres>, op : Transf) {
	atualiza_saldo(&bd, &CreDeb{ user : op.origem.clone(), valor : (op.valor * -1.0 ) }).await;
	atualiza_saldo(&bd, &CreDeb{ user : op.destino.clone(), valor : op.valor }).await;

	let tempo = Local::now()
		.to_rfc3339_opts(SecondsFormat::Secs, false);

	transacao_transferencia( &bd, tempo.as_str(), op.origem.as_str(), op.destino.as_str(), op.valor * -1.0 ).await;
	transacao_transferencia( &bd, tempo.as_str(), op.destino.as_str(), op.origem.as_str(), op.valor ).await;
}

#[get("/extrato/{id}")]
async fn extrato() {

}
