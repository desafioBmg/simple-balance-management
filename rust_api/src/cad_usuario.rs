use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct CadUsuario {
	pub agencia : Option<i32>,
	pub nome : String,
	pub email : Option< String >,
	pub senha : String
}