use serde::Deserialize;

#[derive(Deserialize)]
pub struct ExtratoData {
	pub data_inicio : String,
	pub data_fim : String,
	pub id : String,
}