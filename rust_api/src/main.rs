mod cad_usuario;

use actix_web::{get, put, App, HttpServer, Responder, HttpResponse, HttpRequest};
use actix_web::web::{Data, Json, Path};
use actix_web::http::header::ContentType;

use sqlx::{Pool, Postgres};
use sqlx::postgres::PgPoolOptions;

use anyhow::Result;

use cad_usuario::CadUsuario;
use sqlx::types::Uuid;
use actix_web::cookie::Cookie;

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
			.service( saldo )
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

#[put("/abrirConta")]
async fn abrir_conta( bd : Data<Pool<Postgres>>, req : HttpRequest, usuario: Json<CadUsuario> ) -> impl Responder {

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
	horario int8 NOT NULL,
	conta_origem uuid NOT NULL,
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

#[put("/{id}")]
async fn saldo( bd : Data<Pool<Postgres>>, req : HttpRequest, usuario: Path<String> ) -> impl Responder {

	let user =

	println!("Chegou!!! {:?}", usuario.as_ref());

	// TODO fazer o tratamento de erro para algum problema de comunicação com o BD
	let saldo : (f64, ) = sqlx::query_as("SELECT saldo FROM usuario WHERE id = $1" )
		.bind( usuario.into_inner() )
		.fetch_one( bd.get_ref() ).await
		.expect(format!( "Erro ao consultar o saldo do usuario {}", usuario.as_ref() ).as_str() );

	let mut resp = HttpResponse::Ok()
		.insert_header(ContentType::json() )
		.body( format!( "{{ \"status\" : \"ok\", \"saldo\" : {} }}", saldo.0 ) );

	resp.add_cookie( &Cookie::new("user", usuario.as_ref() ) );

	resp
}

async fn manual_hello() -> impl Responder {
	HttpResponse::Ok().body("Hey there!")
}