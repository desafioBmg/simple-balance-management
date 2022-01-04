use crate::api_service::Data;
use crate::api_service::DataUpdate;
use crate::api_service::DataStatement;
use crate::api_service::DataFind;
use actix_web::{get, post, web, HttpResponse, Responder};

#[get("/get-all")]
async fn get_all_json(app_data: web::Data<crate::AppState>) -> impl Responder {
    let action = app_data.service_manager.api.get_json();
    let result = web::block(move || action).await;
    match result {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(e) => {
            println!("Error while getting, {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/get-by-dates/{param}")]
async fn get_by_dates(app_data: web::Data<crate::AppState>, data: web::Json<DataFind>,  param: web::Path<String>) -> impl Responder {
    let action = app_data.service_manager.api.get_by_dates(&data, &param);
    let result = web::block(move || action).await;
    match result {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(e) => {
            println!("Error while getting, {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/get-by/{param}")]
async fn get_balance(app_data: web::Data<crate::AppState>, param: web::Path<String>) -> impl Responder {
    let action = app_data.service_manager.api.get_by(&param);
    let result = web::block(move || action).await;
    match result {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(e) => {
            println!("Error while getting, {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/add")]
async fn add_account(app_data: web::Data<crate::AppState>, data: web::Json<Data>) -> impl Responder {
    let action = app_data.service_manager.api.create(&data);
    let result = web::block(move || action).await;
    match result {
        Ok(result) => HttpResponse::Ok().json(result.inserted_id),
        Err(e) => {
            println!("Error while getting, {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/update-operation/{param}")]
async fn update_operation(app_data: web::Data<crate::AppState>, data: web::Json<DataUpdate>, param: web::Path<String>) -> impl Responder {
    let action = app_data.service_manager.api.update_operation(&data, &param);
    let result = web::block(move || action).await;
    match result {
        Ok(result) => HttpResponse::Ok().json(result.modified_count),
        Err(e) => {
            println!("Error while getting, {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

// function that will be called on new Application to configure routes for this module
pub fn init(cfg: &mut web::ServiceConfig) {
	cfg.service(get_balance);
    cfg.service(add_account);
    cfg.service(get_all_json);
	cfg.service(update_operation);
	cfg.service(get_by_dates);
}