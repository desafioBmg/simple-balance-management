use bson::{doc, Document};
use mongodb::results::{UpdateResult, InsertOneResult};
use mongodb::{error::Error, Collection};
use serde::{Deserialize, Serialize};
use chrono::prelude::*;
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime};
use chrono::format::ParseError;

extern crate serde;
extern crate serde_json;

#[derive(Debug, Serialize, Deserialize)]
pub struct Data {
    pub account_name: String,
    pub balance: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataStatement {
    pub account_name: String,
    pub operation: String,
    pub previous_balance: i32,
    pub after_balance: i32,
    pub move_date: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataToDeserialize {
    pub _id: Id,
    pub account_name: String,
    pub balance: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Id {
    oid: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataUpdate {
    pub operation: String,
    pub value: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataFind {
    pub initial_date: String,
    pub final_date: String,
}

#[derive(Clone)]
pub struct ApiService {
    collection: Collection,
}

fn data_to_document(data: &Data) -> Document {
    let Data {
        account_name,
        balance,
    } = data;
    doc! {
        "account_name": account_name,
        "balance": balance,
    }
}

fn data_statement_to_document(data: &DataStatement) -> Document {
    let DataStatement {
        account_name,
        operation,
        previous_balance,
        after_balance,
        move_date,
    } = data;

    doc! {
        "account_name": account_name,
        "operation": operation,
        "previous_balance": previous_balance,
        "after_balance": after_balance,
        "move_date": move_date,
    }
}

fn data_update_to_document(data: &DataUpdate) -> Document {
    let DataUpdate {
        operation,
        value,
    } = data;
	
    doc! {
        "operation": operation,
        "value": value,
    }
}

// Functions with quieries to Mongo
impl ApiService {
    pub fn new(collection: Collection) -> ApiService {
        ApiService { collection }
    }

    pub fn create(&self, _data:&Data) -> Result<InsertOneResult, Error> {
        
        let utc: DateTime<Utc> = Utc::now();   

        let _dataStatement = DataStatement {
            account_name: _data.account_name.clone(),
            operation: "creation".to_string(),
            previous_balance: 0,
            after_balance: _data.balance,
            move_date: utc,
        };

        self.collection.insert_one(data_statement_to_document(&_dataStatement), None);

        self.collection.insert_one(data_to_document(_data), None)
    }

    pub fn update_operation(&self, _data:&DataUpdate, _param: &String) -> Result<UpdateResult, Error> {

        let object_param = bson::oid::ObjectId::with_string(_param).unwrap();

        let _cursor = self.collection.find(doc! { "_id": bson::oid::ObjectId::with_string(&object_param.to_string()).unwrap() }, None).expect("Document not found");
		let docs: Vec<_> = _cursor.map(|doc| doc.unwrap()).collect();
        
        let serialized = serde_json::to_string(&docs).unwrap();
        
        let mut data = str::replace(&serialized, "$", "");
        data = str::replace(&data, "[", "");
        data = str::replace(&data, "]", "");

        let utc: DateTime<Utc> = Utc::now();   

        let deserialized:DataToDeserialize = serde_json::from_str(&data).unwrap();

        let new_balance =
            if _data.operation == "credit" {
                deserialized.balance + _data.value
            } else {
                deserialized.balance - _data.value
            };

        let _dataStatement = DataStatement {
            account_name: deserialized.account_name,
            operation: _data.operation.clone(),
            previous_balance: deserialized.balance,
            after_balance: new_balance,
            move_date: utc,
        };

        self.collection.insert_one(data_statement_to_document(&_dataStatement), None);

		self.collection.update_one(doc! { "_id": object_param }, data_to_document(&Data { account_name: _dataStatement.account_name, balance: new_balance}), None)
    }

    pub fn get_json(&self) -> std::result::Result<std::vec::Vec<bson::ordered::OrderedDocument>, mongodb::error::Error> {
        let cursor = self.collection.find(None,None).ok().expect("Failed to execute find.");
        let docs: Vec<_> = cursor.map(|doc| doc.unwrap()).collect();
        Ok(docs)
    }

    pub fn get_by_dates(&self, _data:&DataFind, _param: &String) -> std::result::Result<std::vec::Vec<bson::ordered::OrderedDocument>, mongodb::error::Error> {

        let datetimeInitial = DateTime::parse_from_rfc3339(&_data.initial_date.to_string()).unwrap();
        let datetimeInitial_utc = datetimeInitial.with_timezone(&Utc);

        let datetimeFinal = DateTime::parse_from_rfc3339(&_data.final_date.to_string()).unwrap();
        let datetimeFinal_utc = datetimeFinal.with_timezone(&Utc);

        let cursor = self.collection.find(
            doc! { 
                "account_name": { 
                    "$regex": _param 
                },
                "move_date":{
                    "$gte": datetimeInitial_utc
                },
                "move_date":{
                    "$lte": datetimeFinal_utc
                },
            }, None
        ).ok().expect("Failed to execute find.");

        let docs: Vec<_> = cursor.map(|doc| doc.unwrap()).collect();
        let serialized = serde_json::to_string(&docs).unwrap();
        
        Ok(docs)
    }
	
    pub fn get_by(&self, param: &String) -> std::result::Result<std::vec::Vec<bson::ordered::OrderedDocument>, mongodb::error::Error> {
        let object_param = bson::oid::ObjectId::with_string(param).unwrap();
        
        let _cursor = self.collection.find(doc! { "_id": bson::oid::ObjectId::with_string(&object_param.to_string()).unwrap() }, None).expect("Document not found");
		let docs: Vec<_> = _cursor.map(|doc| doc.unwrap()).collect();
        
        Ok(docs)
    }
}