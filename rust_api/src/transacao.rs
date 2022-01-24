use serde::{Deserialize};

#[derive(Deserialize)]
pub struct Transf {
	pub origem: String,
	pub destino: String,
	pub valor: f64
}

#[derive(Deserialize)]
pub struct CreDeb {
	pub user: String,
	pub valor: f64
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum Transacao {
	CreditoDebito(CreDeb),
	Transferencia(Transf),
}