use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use actix_files::NamedFile;
use mongodb::{Client, options::ClientOptions, bson::{doc, Bson}};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use serde_json::Value;
use tokio::sync::Mutex;
use std::fs::File;
use std::io::BufReader;
use sha2::{Sha256, Digest};
use hex::encode;

fn generate_invite_code(id: String) -> String {
    let mut hasher = Sha256::new();
    hasher.update(id.as_bytes());
    let result = hasher.finalize();
    let hash_hex = encode(result);

    let mut hasher = Sha256::new();
    hasher.update(hash_hex);
    let result = hasher.finalize();

    let hash_hex = encode(result);
    let invite_code = &hash_hex[hash_hex.len() - 6..];

    invite_code.to_string()
}

#[derive(Debug, Deserialize, Serialize)]
struct UserData {
    _id: String,
    username: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
    register_in_game: f64,
    vault: u8,
    upgrades: HashMap<String, u8>
}

#[derive(Debug, Deserialize, Serialize)]
struct TokenData {
    _id: String,
    language: String,
    oxi_tokens_value: u64,
    last_time_update: f64,
    tokens_hour: u64,
    dynamic_fields: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ReferalData {
    _id: String,
    referal_code: String,
    referals: Vec<String>,
}

struct AppState {
    token_collection: mongodb::Collection<TokenData>,
    datauser_collection: mongodb::Collection<UserData>,
    referal_collection: mongodb::Collection<ReferalData>,
    vault_size_constant: HashMap<u8, u32>,
    upgrades_constant: Config
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct QueryUserData {
    id: u64
}

#[derive(Debug, Deserialize, Serialize)]
struct POSTRequest {
    password: String,
    id: u64,
    username: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
    language: String,
    from_referal: Option<String>,
}

#[derive(Debug)]
enum UpdateError {
    DatabaseError,
    NotFound,
}

impl AppState {
    async fn update_tokens_value_vault(&self, id: &str) -> Result<u64, UpdateError> {
        let filter = doc! { "_id": id };
  
        let data_result = self.token_collection.find_one(filter.clone(), None).await;
        let data = match data_result {
            Ok(Some(doc)) => doc,
            Ok(None) => return Err(UpdateError::NotFound),
            Err(_) => return Err(UpdateError::DatabaseError),
        };
    
        let data_result = self.datauser_collection.find_one(filter.clone(), None).await;
        let data_user_2 = match data_result {
            Ok(Some(doc)) => doc,
            Ok(None) => return Err(UpdateError::NotFound),
            Err(_) => return Err(UpdateError::DatabaseError),
        };

        let last_time_update = data.last_time_update;
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs_f64();
        let time_difference = current_time - last_time_update;
        let time_difference_in_hours = time_difference / 3600.0;
        let added_tokens = (time_difference_in_hours * 1000.0) as u64;
        let vault_size = self.vault_size_constant[&data_user_2.vault] as u64;
    
        if added_tokens > vault_size {
            return Ok(vault_size);
        }
        
        Ok(added_tokens)
    }
}

async fn index() -> impl Responder {
    NamedFile::open_async("./templates/index.html").await.unwrap()
}

async fn create_new_account(
    state: web::Data<Mutex<AppState>>, 
    data: web::Json<POSTRequest>
) -> impl Responder {

    let password = "123";
    if data.password != password {
        let error = ErrorResponse { error: "Auth error".to_string() };
        return HttpResponse::BadRequest().json(error);
    }

    let state = state.lock().await;

    let result = state.token_collection.count_documents(doc! {"_id": data.id.to_string()}, None).await;
    let count = match result {
        Ok(count) => count,
        Err(e) => {
            eprintln!("Failed to count documents: {:?}", e);
            return HttpResponse::InternalServerError().body("Internal Server Error");
        }
    };

    if count > 0 {
        return HttpResponse::Ok().body("{'code':0,'msg':'User already register'}");
    }

    let last_time_update = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs_f64(),
        Err(_) => {
            let error = ErrorResponse { error: "Failed to get current time".to_string() };
            return HttpResponse::InternalServerError().json(error);
        }
    };

    let user_data = UserData {
        _id: data.id.to_string(),
        username: data.username.clone(),
        first_name: data.first_name.clone(),
        last_name: data.last_name.clone(),
        register_in_game: last_time_update,
        vault: 1,
        upgrades: HashMap::new()
    };

    let mut token_data = TokenData {
        _id: data.id.to_string(),
        language: data.language.clone(),
        oxi_tokens_value: 0,
        last_time_update: last_time_update,
        tokens_hour: 1000,
        dynamic_fields: None
    };

    let referal_data = ReferalData {
        _id: data.id.to_string(),
        referal_code: generate_invite_code(data.id.to_string()),
        referals: Vec::new(),
    };

    match &data.from_referal {
        // Извлечение значения referal_code
        Some(referal_code_from_data) => {
            // Поиск рефераловода
            match state.referal_collection.find_one(doc! { "referal_code": referal_code_from_data }, None).await {
                Ok(Some(mut d)) => {
                    // Новый реферал, добавление его id
                    d.referals.push(data.id.to_string());
                    // Подготовка данных для обновления
                    let update_doc = doc! { "$set": { "referals": &d.referals } };
                    match state.referal_collection.update_one(doc! { "referal_code": referal_code_from_data }, update_doc, None).await {
                        Ok(_) => {}
                        Err(_) => {
                            let error = ErrorResponse { error: "Failed to update document".to_string() };
                            return HttpResponse::InternalServerError().json(error);
                        }
                    }
                    // Реферал добавлен
                    
                    // Поиск данных рфераловода
                    let mut data_collection_value = match state.token_collection.find_one(doc! { "_id": &d._id }, None).await {
                        Ok(Some(d)) => d,
                        Ok(None) => {
                            let error = ErrorResponse { error: "User not found".to_string() };
                            return HttpResponse::NotFound().json(error);
                        }
                        Err(_) => {
                            let error = ErrorResponse { error: "Database query failed".to_string() };
                            return HttpResponse::InternalServerError().json(error);
                        }
                    };
                    
                    data_collection_value.oxi_tokens_value += 25000;
                    token_data.oxi_tokens_value += 25000;

                    let update_doc = doc! { "$set": { "oxi_tokens_value": Bson::from(data_collection_value.oxi_tokens_value as i64) } };
                    match state.token_collection.update_one(doc! { "_id": &d._id }, update_doc, None).await {
                        Ok(_) => {},
                        Err(err) => {
                            println!("{}", err);
                            let error = ErrorResponse { error: "Failed to insert data in database".to_string() };
                            return HttpResponse::InternalServerError().json(error);
                        }
                    }
                }
                Ok(None) => {}
                Err(_) => {
                    let error = ErrorResponse { error: "Database query failed".to_string() };
                    return HttpResponse::InternalServerError().json(error);
                }
            };
        }
        None => {}
    };

    match state.token_collection.insert_one(token_data, None).await {
        Ok(_) => {},
        Err(err) => {
            println!("{}", err);
            let error = ErrorResponse { error: "Failed to insert data in database".to_string() };
            return HttpResponse::InternalServerError().json(error);
        }
    };

    match state.datauser_collection.insert_one(user_data, None).await {
        Ok(_) => {},
        Err(err) => {
            println!("{}", err);
            let error = ErrorResponse { error: "Failed to insert data in database".to_string() };
            return HttpResponse::InternalServerError().json(error);
        }
    };

    match state.referal_collection.insert_one(referal_data, None).await {
        Ok(_) => {},
        Err(err) => {
            println!("{}", err);
            let error = ErrorResponse { error: "Failed to insert data in database".to_string() };
            return HttpResponse::InternalServerError().json(error);
        }
    };

    HttpResponse::Ok().body("{'code':1,'msg':'OK'}")
}

async fn get_data(
    state: web::Data<Mutex<AppState>>, 
    query: web::Json<QueryUserData>
) -> impl Responder {
    let id = query.id.to_string();

    let state = state.lock().await;
    let mut data = match state.token_collection.find_one(doc! { "_id": &id }, None).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            let error = ErrorResponse { error: "User not found".to_string() };
            return HttpResponse::NotFound().json(error);
        }
        Err(_) => {
            let error = ErrorResponse { error: "Database query failed".to_string() };
            return HttpResponse::InternalServerError().json(error);
        }
    };

    let data_user_improvements = match state.datauser_collection.find_one(doc! { "_id": &id }, None).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            let error = ErrorResponse { error: "User not found".to_string() };
            return HttpResponse::NotFound().json(error);
        }
        Err(_) => {
            let error = ErrorResponse { error: "Database query failed".to_string() };
            return HttpResponse::InternalServerError().json(error);
        }
    };

    let data_referal = match state.referal_collection.find_one(doc! { "_id": &id }, None).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            let error = ErrorResponse { error: "User not found".to_string() };
            return HttpResponse::NotFound().json(error);
        }
        Err(_) => {
            let error = ErrorResponse { error: "Database query failed".to_string() };
            return HttpResponse::InternalServerError().json(error);
        }
    };

    let added_tokens = match state.update_tokens_value_vault(&id).await {
        Ok(tokens) => tokens,
        Err(_) => {
            let error = ErrorResponse { error: "Failed to update token values".to_string() };
            return HttpResponse::InternalServerError().json(error);
        }
    };
    println!("{}", &data_referal.referal_code);
    let mut dynamic_data = HashMap::new();
    let vault_use = (data.oxi_tokens_value as u64 / state.vault_size_constant[&data_user_improvements.vault] as u64 * 100) as i32;
    dynamic_data.insert("added_tokens".to_string(), added_tokens.to_string());
    dynamic_data.insert("vault_use".to_string(), vault_use.to_string());
    dynamic_data.insert("vault_size".to_string(), state.vault_size_constant[&data_user_improvements.vault].to_string());
    dynamic_data.insert("referal_code".to_string(), data_referal.referal_code);
    dynamic_data.insert("referals_value".to_string(), data_referal.referals.len().to_string());

    data.dynamic_fields = Some(dynamic_data);

    HttpResponse::Ok().json(data)
}

async fn claim_tokens(
    state: web::Data<Mutex<AppState>>, 
    query: web::Query<HashMap<String, String>>
) -> impl Responder {
    let json_str = match query.get("user") {
        Some(s) => s,
        None => {
            let error = ErrorResponse { error: "Missing 'user' query parameter".to_string() };
            return HttpResponse::BadRequest().json(error);
        }
    };

    let json_val: Value = match serde_json::from_str(json_str) {
        Ok(val) => val,
        Err(_) => {
            let error = ErrorResponse { error: "Failed to parse JSON".to_string() };
            return HttpResponse::BadRequest().json(error);
        }
    };

    let id = match json_val.get("id").and_then(|v| v.as_u64()) {
        Some(id) => id.to_string(),
        None => {
            let error = ErrorResponse { error: "Missing or invalid 'id' in JSON".to_string() };
            return HttpResponse::BadRequest().json(error);
        }
    };
    let state = state.lock().await;

    let mut data = match state.token_collection.find_one(doc! { "_id": &id }, None).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            let error = ErrorResponse { error: "User not found".to_string() };
            return HttpResponse::NotFound().json(error);
        }
        Err(_) => {
            let error = ErrorResponse { error: "Database query failed".to_string() };
            return HttpResponse::InternalServerError().json(error);
        }
    };

    let data_user_improvements = match state.datauser_collection.find_one(doc! { "_id": &id }, None).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            let error = ErrorResponse { error: "User not found".to_string() };
            return HttpResponse::NotFound().json(error);
        }
        Err(_) => {
            let error = ErrorResponse { error: "Database query failed".to_string() };
            return HttpResponse::InternalServerError().json(error);
        }
    };
    
    let added_tokens = match state.update_tokens_value_vault(&id).await {
        Ok(tokens) => tokens,
        Err(_) => {
            let error = ErrorResponse { error: "Failed to update token values".to_string() };
            return HttpResponse::InternalServerError().json(error);
        }
    };

    data.oxi_tokens_value += added_tokens as u64;
    let last_time_update = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs_f64(),
        Err(_) => {
            let error = ErrorResponse { error: "Failed to get current time".to_string() };
            return HttpResponse::InternalServerError().json(error);
        }
    };
    data.last_time_update = last_time_update;

    match state.token_collection.replace_one(doc! { "_id": &id }, &data, None).await {
        Ok(_) => {}
        Err(_) => {
            let error = ErrorResponse { error: "Failed to replace data in database".to_string() };
            return HttpResponse::InternalServerError().json(error);
        }
    }
    let mut dynamic_data = HashMap::new();
    let vault_use = (data.oxi_tokens_value as u64 / state.vault_size_constant[&data_user_improvements.vault] as u64 * 100) as i32;
    dynamic_data.insert("added_tokens".to_string(), added_tokens.to_string());
    dynamic_data.insert("vault_use".to_string(), vault_use.to_string());
    dynamic_data.insert("vault_size".to_string(), state.vault_size_constant[&data_user_improvements.vault].to_string());

    data.dynamic_fields = Some(dynamic_data);
    
    HttpResponse::Ok().json(data)
}


#[derive(Debug, Deserialize, Serialize)]
struct UpdateData {
    _id: String,
    type_update: String,
    id_update: String
}
// USER DATA
// {"miner_1": 12, "miner_2": 3, "miner_3": 2, "miner_4": 1}

// UPGARDE DATA
// {"miner": 1: {"buy_price": 13121}}

async fn update(
    state: web::Data<Mutex<AppState>>, 
    data: web::Json<UpdateData>
) -> impl Responder {
    // data.type_update => miner, vault
    // data.id_update => miner_1, miner_2.. vault_main - значения созхранены в бд user_data

    // Запрос на повышение уровня на 1 единицу некоторого объекта
    let state = state.lock().await;
    // Получение данных пользователя по его id (USER DATA)
    let data_user = match state.datauser_collection.find_one(doc! { "_id": &data._id }, None).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            let error = ErrorResponse { error: "User not found".to_string() };
            return HttpResponse::NotFound().json(error);
        }
        Err(_) => {
            let error = ErrorResponse { error: "Database query failed".to_string() };
            return HttpResponse::InternalServerError().json(error);
        }
    };
    // Получение текущего уровня объекта
    // let last_level_upgrade = data_user.upgrades.get(&data.id_update) + 1;

    let last_level_upgrade = match data_user.upgrades.get(&data.id_update) {
        Some(level) => (level + 1).to_string(),
        None => {
            let error = ErrorResponse { error: "User not found".to_string() };
            return HttpResponse::NotFound().json(error);
        } 
    };

    // // Получение данных что нужно для следюущего уровня
    let new_level_data = if &data.type_update == "miner" {
        Some(state.upgrades_constant.miner.get(&last_level_upgrade).unwrap())
    } else {
        None
    };

    let mut token_data = match state.token_collection.find_one(doc! { "_id": &data._id }, None).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            let error = ErrorResponse { error: "User not found".to_string() };
            return HttpResponse::NotFound().json(error);
        }
        Err(_) => {
            let error = ErrorResponse { error: "Database query failed".to_string() };
            return HttpResponse::InternalServerError().json(error);
        }
    };

    if token_data.oxi_tokens_value < new_level_data.unwrap().buy_price {
        let error = ErrorResponse { error: "Insufficient balance".to_string() };
        return HttpResponse::BadRequest().json(error);
    } 
    
    token_data.oxi_tokens_value -= new_level_data.unwrap().buy_price;
    token_data.tokens_hour += new_level_data.unwrap().tokens_add;


    HttpResponse::Ok().json(token_data)
}

#[derive(Serialize, Deserialize, Debug)]
struct MinerConfig {
    buy_price: u64,
    tokens_add: u64,
}


#[derive(Serialize, Deserialize, Debug)]
struct Config {
    miner: HashMap<String, MinerConfig>,
    buy_miner: HashMap<String, u64>
}


fn load_config(file_path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let config = serde_json::from_reader(reader)?;
    Ok(config)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let client_options = ClientOptions::parse("mongodb://localhost:27017").await.unwrap();
    let db_client = Client::with_options(client_options).unwrap();
    let db = db_client.database("OXI");
    let token_collection = db.collection::<TokenData>("OXI_tokens");
    let datauser_collection = db.collection::<UserData>("OXI_improvements");
    let referal_collection = db.collection::<ReferalData>("OXI_referals");

    let vault_size_constant = HashMap::from([
        (1, 5000), (2, 12000), (3, 50000), (4, 120000), 
        (5, 450000), (6, 800000), (7, 1600000), 
        (8, 3500000), (9, 5000000), (10, 10000000)
    ]);
    
    let upgrades_constant = match load_config("config/config.json") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error loading config: {}", e);
            panic!("Fail load config");
        } 
    };

    let state = web::Data::new(Mutex::new(AppState { 
        token_collection, 
        datauser_collection, 
        referal_collection,
        vault_size_constant,
        upgrades_constant
    }));

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .route("/", web::get().to(index))
            .route("/api/data", web::post().to(get_data))
            .route("/api/update", web::post().to(update))
            .route("/claim_tokens", web::get().to(claim_tokens))
            .route("/newaccount", web::post().to(create_new_account))
            .service(actix_files::Files::new("/static", "./static").show_files_listing())
    })
    .bind(("127.0.0.1", 8081))?
    .run()
    .await
}