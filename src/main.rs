use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use actix_files::NamedFile;
use mongodb::{Client, options::ClientOptions, bson::{doc, Bson}};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use serde_json::Value;

// #[derive(Serialize, Deserialize)]
// struct DataOXITokens {
//     _id: u128,
//     language: String,
//     user_name: String,
//     oxi_tokens_value: u128,
//     last_time_update: u128
// }

// struct Main {
//     client: Client,
//     db: Database,
//     collections_TOKENS: Collection<DataOXITokens>,
//     collections_IMPROVEMENTS: Collection<DataOXIImprovemets>,
//     collections_USER: Collection<>,
//     vaultSize: HashMap<u8, u32>,
// }


#[derive(Debug, Deserialize, Serialize)]
struct TokenData {
    _id: String,
    last_time_update: f64,
    oxi_tokens_value: u128,
}

#[derive(Debug, Deserialize, Serialize)]
struct ImprovementData {
    _id: String,
    storage: u8,
    vault: u8,
}

struct AppState {
    token_collection: mongodb::Collection<TokenData>,
    improvement_collection: mongodb::Collection<ImprovementData>,
    vault_size_constant: HashMap<u8, u32>,
}

impl AppState {
    async fn update_tokens_value_vault(&self, id: &str) -> u32 {
        let filter = doc! { "_id": id };
        let data = self.token_collection.find_one(filter.clone(), None).await.unwrap().unwrap();
        let data_user_2 = self.improvement_collection.find_one(filter, None).await.unwrap().unwrap();

        let last_time_update = data.last_time_update;
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs_f64();
        let time_difference = current_time - last_time_update;
        let time_difference_in_hours = time_difference / 3600.0;
        let added_tokens = (time_difference_in_hours * 1000.0) as u32;
        let vault_size = self.vault_size_constant[&data_user_2.vault];

        if added_tokens > vault_size {
            return vault_size;
        }
        added_tokens
    }
}

async fn index() -> impl Responder {
    NamedFile::open_async("./templates/index.html").await.unwrap()
}

async fn friends() -> impl Responder {
    NamedFile::open_async("./templates/friends.html").await.unwrap()
}

async fn get_data(state: web::Data<Mutex<AppState>>, query: web::Query<HashMap<String, String>>) -> impl Responder {
    let json_str = &query.get("user").unwrap();
    println!("{}", json_str);
    let json_val: Value = serde_json::from_str(json_str).expect("Не удалось распарсить json!");
    println!("{}", json_val);
    let id= json_val.get("id").expect("КЛЮЧА НЕТ").as_str().unwrap();
    println!("{}", id);
    let state = state.lock().unwrap();
    let data = state.token_collection.find_one(doc! { "_id": id }, None).await.unwrap().unwrap();

    let added_tokens = state.update_tokens_value_vault(id).await;
    let mut data = data;
    data.oxi_tokens_value += added_tokens as u128;
    
    HttpResponse::Ok().json(data)
}

async fn get_counter(state: web::Data<Mutex<AppState>>, query: web::Query<HashMap<String, String>>) -> impl Responder {
    let json_str = &query.get("user").unwrap();
    let json_val: Value = serde_json::from_str(json_str).unwrap();
    let id = &json_val["id"].as_str().unwrap();
    let state = state.lock().unwrap();
    let added_tokens = state.update_tokens_value_vault(id).await;
    
    HttpResponse::Ok().body(added_tokens.to_string())
}

async fn update_counter(state: web::Data<Mutex<AppState>>, query: web::Query<HashMap<String, String>>) -> impl Responder {
    let json_str = &query.get("user").unwrap();
    let json_val: Value = serde_json::from_str(json_str).unwrap();
    let id = &json_val["id"].as_str().unwrap();
    let state = state.lock().unwrap();
    let data = state.token_collection.find_one(doc! { "_id": id }, None).await.unwrap().unwrap();
    
    HttpResponse::Ok().body(data.oxi_tokens_value.to_string())
}

async fn claim_tokens(state: web::Data<Mutex<AppState>>, query: web::Query<HashMap<String, String>>) -> impl Responder {
    let json_str = &query.get("user").unwrap();
    let json_val: Value = serde_json::from_str(json_str).unwrap();
    let id = &json_val["id"].as_str().unwrap();
    let state = state.lock().unwrap();
    let filter = doc! { "_id": id };
    let mut data = state.token_collection.find_one(filter.clone(), None).await.unwrap().unwrap();
    let data_user_2 = state.improvement_collection.find_one(filter.clone(), None).await.unwrap().unwrap();

    let added_tokens = state.update_tokens_value_vault(id).await;
    data.oxi_tokens_value += added_tokens as u128;
    data.last_time_update = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs_f64();
    let vault_use = (data.oxi_tokens_value as f64 / state.vault_size_constant[&data_user_2.vault] as f64 * 100.0) as i32;

    state.token_collection.replace_one(filter.clone(), &data, None).await.unwrap();
    let mut response = serde_json::to_value(data).unwrap();
    response.as_object_mut().unwrap().insert("vault_use".to_string(), Bson::Int32(vault_use).into());

    HttpResponse::Ok().json(response)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let client_options = ClientOptions::parse("mongodb://localhost:27017").await.unwrap();
    let db_client = Client::with_options(client_options).unwrap();
    let db = db_client.database("OXI");
    let token_collection = db.collection::<TokenData>("OXI_tokens");
    let improvement_collection = db.collection::<ImprovementData>("OXI_improvements");

    let vault_size_constant = HashMap::from([
        (1, 5000), (2, 12000), (3, 50000), (4, 120000), 
        (5, 450000), (6, 800000), (7, 1600000), 
        (8, 3500000), (9, 5000000), (10, 10000000)
    ]);

    let state = web::Data::new(Mutex::new(AppState { 
        token_collection, 
        improvement_collection, 
        vault_size_constant 
    }));

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .route("/", web::get().to(index))
            .route("/friends", web::get().to(friends))
            .route("/getdata", web::get().to(get_data))
            .route("/get_counter", web::get().to(get_counter))
            .route("/update_counter", web::get().to(update_counter))
            .route("/claim_tokens", web::get().to(claim_tokens))
            .service(actix_files::Files::new("/static", "./static").show_files_listing())
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}