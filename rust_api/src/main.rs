mod cad_usuario;

use actix_web::{get, App, HttpServer, Responder, HttpResponse, HttpRequest, HttpResponseBuilder};
use actix_web::web::{ Data, Form };
use sqlx::postgres::PgPoolOptions;
use anyhow::Result;
use sqlx::{Postgres, Pool};
use actix_web::body::BoxBody;
use cad_usuario::Cad_Usuario;

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
			.app_data( pool.clone() )
			.service(teste )
			.service( status )
	} ).bind("0.0.0.0:7777")?
		.run()
		.await;

	println!("Hello, world!");

	Ok(())
}

#[get("/")]
async fn teste( req: HttpRequest ) -> impl Responder {
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
async fn abrir_conta( bd : Data<PgPoolOptions>, form : form: Form<Cad_Usuario> ) -> impl Responder {
	let ag = agencia.unwrap_or( 1 );

	bd.

	// TODO fazer o tratamento de erro para algum problema de comunicação com o BD
	let conta (i64, ) = (sqlx::query("SELECT coalesce( MAX( conta ), 1) FROM usuario WHERE agencia = $1" )
		.bind( ag)
		.fetch_one( bd.clone() ).await?;

	// TODO fazer o tratamento de erro para algum problema de comunicação com o BD
	let resultado: (i32, &str) = sqlx::query("INSERT INTO usuario (agencia, conta, senha, nome, email) VALUES ( $1, $2, $3, $4, $5 )")
		.bind(ag )
		.bind( conta )
		.fetch( bd.clone() ).await?;

	// TODO fazer tratamento de erro para a recuperação da chave do usuário com o BD
	let chave : (&str, ) = sqlx::query("SELECT id FROM usuario WHERE agencia = $1 AND conta = $2")
		.bind( ag )
		.bind( conta )
		.fetch_one( bd.clone() ).await?;

	// TODO consultar a chave do cliente e criar uma tabela com o nome da chave para armazenar as transações
	let _ = sqlx::query("CREATE TABLE public.\"f177fc66-e2fc-46e0-b702-97fff3581329_T\" (
	horario int8 NOT NULL,
	conta_origem uuid NOT NULL,
	valor numeric NOT NULL
)");

	resposta_api_sucesso(format!( "{{ status : 'ok', agencia : {}, conta_corrente : {}, nome : '{}' }}", ag, conta, "Teste"))
}

async fn manual_hello() -> impl Responder {
	HttpResponse::Ok().body("Hey there!")
}

fn reposta_api_sucesso( json : &str ) -> HttpResponse<BoxBody> {
	HttpResponse::Ok()
		.append_header(("Content-Type", "application/json"))
		.body( json )
}
