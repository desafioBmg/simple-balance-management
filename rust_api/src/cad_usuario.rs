use serde::Deserialize;

#[derive(Deserialize)]
pub struct Cad_Usuario {
	agencia : Option<i32>,
	nome : String,
	email : Option< String >,
	senha : String
}