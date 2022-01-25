mod cad_usuario;
mod transacao;
mod extrato_data;

use actix_web::{get, post, App, HttpServer, Responder, HttpResponse, HttpRequest};
use actix_web::web::{Data, Json, Path};
use actix_web::http::header::ContentType;
use actix_web::cookie::Cookie;

use sqlx::{Pool, Postgres, Row};
use sqlx::postgres::PgPoolOptions;
use sqlx::types::Uuid;

use anyhow::Result;

use chrono::{Local, SecondsFormat};

use crate::cad_usuario::CadUsuario;
use crate::transacao::Transacao::{CreditoDebito, Transferencia};
use crate::transacao::{Transacao, Transf};
use crate::transacao::CreDeb;
use crate::extrato_data::ExtratoData;

#[tokio::main]
async fn main() -> Result<()> {

	let pool = PgPoolOptions::new()
		.max_connections(5)
		.connect("postgres://postgres:SuperSecreto@localhost/bd_bmg").await?;

	println!("Inicializando o Servidor");

	let _ = HttpServer::new( move || {
		App::new()
			.app_data( Data::new( pool.clone() ) )
			.service(teste )
			.service( status )
			.service( abrir_conta )
			.service( balanco )
			.service( transacoes )
			.service( extrato )
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
	horario varchar NOT NULL,
	conta uuid,
	valor float8 NOT NULL
)", chave.0 ).as_str() )
		.execute(bd.get_ref() ).await
		.expect("Erro ao liberar a estrutura de transações do usuário.");

	let mut resp = HttpResponse::Ok()
		.insert_header(ContentType::json() )
		.body( format!( "{{ \"status\" : \"ok\", \"agencia\" : {}, \"conta_corrente\" : {}, \"nome\" : \"{}\" }}", ag, conta.0, usuario.nome) );

	let _ = resp.add_cookie( &Cookie::new("user", chave.0.to_string()));

	resp
}

#[get("/{id}")]
async fn balanco( bd : Data<Pool<Postgres>>, usuario: Path<String> ) -> impl Responder {

	let user = Uuid::parse_str( usuario.as_str() ).expect("Código inválido!");

	// TODO fazer o tratamento de erro para algum problema de comunicação com o BD
	let saldos : (f64, ) = sqlx::query_as("SELECT saldo FROM usuario WHERE id = $1" )
		.bind( user )
		.fetch_one( bd.get_ref() ).await
		.expect(format!( "Erro ao consultar o saldo do usuario {}", usuario.as_str() ).as_str());

	let mut resp = HttpResponse::Ok()
		.insert_header(ContentType::json() )
		.body( format!( "{{ \"status\" : \"ok\", \"saldo\" : {} }}", saldos.0 ) );

	let _ = resp.add_cookie( &Cookie::new("user", usuario.as_str() ) );

	resp
}

#[post("/credito")]
async fn transacoes(bd : Data<Pool<Postgres>>, transacao: Json<Transacao> ) -> impl Responder {

	match transacao.0 {
		CreditoDebito(op ) => {
			if valida_saldo(bd.get_ref().clone(), op.user.as_str(), op.valor ).await {
				transacao_basica( &bd, &op ).await;
			}

		},
		Transferencia( op ) => {
			if op.valor > 0.0 && valida_saldo(bd.get_ref().clone(), op.origem.as_str(), op.valor * -1.0 ).await {
				transferencia( &bd, &op ).await;
			} else {
				return HttpResponse::Ok()
					.content_type(ContentType::json())
					.body( format!("{{ \"status\" : \"erro\", \"motivo\" : \"Valor inválido.\" }}") )
			}

		}
	}

	// TODO Gerar a mensagem de confirmação da transação;
	HttpResponse::Ok()
		.content_type(ContentType::json() )
		.body(format!("{{ \"status\" : \"ok\" }}") )
}

async fn valida_saldo(bd : Pool<Postgres>, usuario : &str, valor : f64 ) -> bool {

	let user = Uuid::parse_str( usuario ).expect("Código inválido");

	let saldo : (f64, ) = sqlx::query_as("SELECT saldo FROM usuario WHERE id = $1" )
		.bind( user )
		.fetch_one( &bd ).await
		.expect("Erro ao definiro número da conta");

	if valor > 0.0 || ( saldo.0 + valor ) >= 0f64 {
		atualiza_saldo(&bd, &CreDeb{ user: usuario.to_owned(), valor : ( saldo.0 + valor ) } ).await;
		return true;
	}

	false
}

async fn atualiza_saldo(bd : &Pool<Postgres>, op : &CreDeb ) {
	let user = Uuid::parse_str( op.user.as_str() ).expect("Código inválido");

	sqlx::query("UPDATE usuario SET saldo = $1 WHERE id = $2" )
		.bind( op.valor )
		.bind( user )
		.execute( &*bd ).await
		.expect( "[atualiza_saldo] : Erro ao atualizar o saldo da transação." );
}

async fn transacao_basica(bd : &Pool<Postgres>, op : &CreDeb ) {

	// ! Deve ser feita as checagens para evitar sql injection;
	let sql = format!("INSERT INTO \"{}_T\"( horario, valor ) values ( '{}', {} )", op.user, retona_horario().as_str(), op.valor);

	sqlx::query( sql.as_str() )
		.execute( &*bd ).await
		.expect( "[transacao_basica] : Erro ao atualizar o saldo da transação." );
}

async fn transacao_transferencia( bd : &Pool<Postgres>, tempo : &str, conta1 : &str, conta2 : &str, valor : f64 ) {

	// ! Deve ser feita as checagens para evitar sql injection;
	let sql = format!("INSERT INTO \"{}_T\"( horario, valor, conta ) values ( '{}', {}, '{}' )", conta1, tempo, valor, conta2 );

	sqlx::query( sql.as_str() )
		.execute( &*bd ).await
		.expect( "Erro ao atualizar o saldo da transação." );
}

async fn transferencia(   bd : &Pool<Postgres>, op : &Transf) {
	atualiza_saldo(&bd, &CreDeb{ user : op.destino.clone(), valor : op.valor }).await;

	let tempo = retona_horario();

	transacao_transferencia( &bd, tempo.as_str(), op.origem.as_str(), op.destino.as_str(), op.valor * -1.0 ).await;
	transacao_transferencia( &bd, tempo.as_str(), op.destino.as_str(), op.origem.as_str(), op.valor ).await;
}

#[get("/extrato/{id}/{data_inicio}/{data_fim}")]
async fn extrato( bd : Data<Pool<Postgres>>, info: Path<ExtratoData>) -> impl Responder {
	let sql = format!("select
	SUBSTRING( t.horario, 1, 19) as \"Horário\",
	case when t.conta is not null then concat('ag:', u.agencia, ' / cc:', u.conta) else '' end as \"conta\",
	t.valor
from \"{}_T\" t
	left join usuario u  on ( t.conta = u.id )
where SUBSTRING( horario, 1, 10) between '{}' and '{}'", info.id, info.data_inicio, info.data_fim );

	let  res_linhas = sqlx::query( sql.as_str() )
		.fetch_all( bd.get_ref() ).await;

	let mut json_resp = String::from('[');

	if let Ok( linhas ) = res_linhas {
		for l in linhas {
			let horario : &str = l.get::<&str, usize>(0);
			let conta : &str = l.get::<&str, usize>(1);
			let valor : f64 = l.get::<f64, usize>(2);

			json_resp.push_str( format!("{{ \"horario\" : {}, \"conta\" : {}, \"valor\" : {} }}", horario, conta, valor ).as_str() );
		}

	}

	json_resp.push(']');

	HttpResponse::Ok()
		.content_type( ContentType::json() )
		.body( json_resp )
}

fn retona_horario() -> String {
	Local::now()
		.to_rfc3339_opts(SecondsFormat::Secs, false)
}
