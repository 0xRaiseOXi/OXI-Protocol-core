use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use actix_files::NamedFile;
use mongodb::{Client, options::ClientOptions, bson::{doc, Bson}};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
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
struct TokenData {
    _id: String,
    register_in_game: f64,
    upgrades: HashMap<String, u8>,
    oxi_tokens_value: u64,
    last_time_update: f64,
    tokens_hour: u32,
    referal_code: String,
    referals: Vec<String>,
    level: u32,
}

#[derive(Debug, Deserialize, Serialize)]
struct MainResponse {
    _id: String,
    upgrades_current: HashMap<String, HashMap<String, String>>,
    upgrades_new: HashMap<String, HashMap<String, String>>,
    oxi_tokens_value: u64,
    last_time_update: f64,
    tokens_hour: u32,
    referal_code: String,
    referals: Vec<String>,
    referals_value: String,
    level: u32,
    war: u64,
}


impl TokenData {
    fn build_response(&self, state: &AppState, data: &TokenData) -> Result<MainResponse, HttpResponse> {
        let mut upgrades_chapshot = HashMap::new();
        let mut upgrades_chapshot_new = HashMap::new();

        for (key, b) in &data.upgrades {
            let mut upgrades_local = HashMap::new();
            let mut upgrades_new = HashMap::new();

            let current_level_upgrade = match state.upgrades_constant.miner.get(&b.to_string()) {
                Some(v) => v,
                None => {
                    let error = ErrorResponse { error: "Failed to Data Base".to_string() };
                    return Err(HttpResponse::InternalServerError().json(error));
                }
            };

            let new_level_upgrade = match state.upgrades_constant.miner.get(&(b + 1).to_string()) {
                Some(v) => v,
                None => {
                    let error = ErrorResponse { error: "Failed to Data Base".to_string() };
                    return Err(HttpResponse::InternalServerError().json(error));    
                }
            };

            let mut tokens_add = current_level_upgrade.tokens_add;
            let mut new_tokens_add = new_level_upgrade.tokens_add;
            let mut buy_price = new_level_upgrade.buy_price;

            if key == "miner_2" {
                tokens_add += tokens_add * 2 / 10;
                new_tokens_add += new_tokens_add * 2 / 10;
                buy_price += buy_price * 2 / 10;
            } else if key == "miner_3" {
                tokens_add += tokens_add * 3 / 10;
                new_tokens_add += new_tokens_add * 3 / 10;
                buy_price += buy_price * 3 / 10;
            } else if key == "miner_4" {
                tokens_add += tokens_add * 4 / 10;
                new_tokens_add += new_tokens_add * 4 / 10;
                buy_price += buy_price * 4 / 10;
            }

            upgrades_local.insert("tokens_hour".to_string(), tokens_add.to_string());
            upgrades_local.insert("level".to_string(), b.to_string());

            upgrades_new.insert("tokens_hour".to_string(), new_tokens_add.to_string());
            upgrades_new.insert("price".to_string(), buy_price.to_string());

            upgrades_chapshot.insert(key.to_string(), upgrades_local.clone());
            upgrades_chapshot_new.insert(key.to_string(), upgrades_new.clone());
        }
        
        let mut war = 0;
        war += self.oxi_tokens_value / 10000;
        war += self.level as u64;
        war += self.referals.len() as u64 * 100;

        let main_response = MainResponse {
            _id: self._id.clone(),
            upgrades_current: upgrades_chapshot,
            upgrades_new: upgrades_chapshot_new,
            oxi_tokens_value: self.oxi_tokens_value,
            last_time_update: self.last_time_update,
            tokens_hour: self.tokens_hour,
            referal_code: self.referal_code.clone(),
            referals: self.referals.clone(),
            referals_value: self.referals.len().to_string(),
            level: self.level,
            war: war
        };

        Ok(main_response)
    }
}

struct AppState {
    token_collection: mongodb::Collection<TokenData>,
    upgrades_constant: Config,
    password: String
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct QueryUserData {
    id: u64
}

#[derive(Debug)]
enum UpdateError {
    DatabaseError,
    NotFound,
}

#[derive(Debug, Serialize)]
struct SuccessResponse {
    msg: String,
}

impl AppState {
    async fn update_tokens_value_vault(&self, id: &str) -> Result<u64, UpdateError> {
        let data_result = self.token_collection.find_one(doc! { "_id": id }, None).await;
        let data = match data_result {
            Ok(Some(doc)) => doc,
            Ok(None) => return Err(UpdateError::NotFound),
            Err(_) => return Err(UpdateError::DatabaseError),
        };
        
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs_f64();

        let time_difference = current_time - data.last_time_update;
        let time_difference_in_hours = time_difference / 3600.0;
        let added_tokens = (time_difference_in_hours * data.tokens_hour as f64) as u64;
    
        if time_difference > 8.0 * 60.0 * 60.0 {
            return Ok((data.tokens_hour * 8) as u64);
        }
        
        Ok(added_tokens)
    }
}

async fn index() -> impl Responder {
    NamedFile::open_async("./templates/index.html").await.unwrap()
}

#[derive(Debug, Deserialize, Serialize)]
struct RequestRegister {
    password: String,
    id: u64,
    username: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
    language: String,
    from_referal: Option<String>,
}


async fn create_new_account(
    guard: web::Data<Mutex<AppState>>, 
    data: web::Json<RequestRegister>
) -> impl Responder {
    let state = guard.lock().await;

    if data.password != state.password {
        let error = ErrorResponse { error: "Auth error".to_string() };
        return HttpResponse::BadRequest().json(error);
    }

    match state.token_collection.count_documents(doc! {"_id": data.id.to_string()}, None).await {
        Ok(count) => {
            if count > 0 {
                let error = ErrorResponse { error: "User already register".to_string() };
                return HttpResponse::BadRequest().json(error);
            }
            count
        }
        Err(e) => {
            eprintln!("Failed to count documents: {:?}", e);
            let error = ErrorResponse { error: "Internal Server Error".to_string() };
            return HttpResponse::InternalServerError().json(error);
        }
    };

    let last_time_update = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs_f64(),
        Err(_) => {
            let error = ErrorResponse { error: "Failed to get current time".to_string() };
            return HttpResponse::InternalServerError().json(error);
        }
    };

    let upgrades: HashMap<String, u8> = HashMap::from([
        ("miner_1".to_string(), 1),
        ("vault_main".to_string(), 1)
    ]);

    let mut token_data = TokenData {
        _id: data.id.to_string(),
        register_in_game: last_time_update,
        upgrades: upgrades,
        oxi_tokens_value: 0,
        last_time_update: last_time_update,
        tokens_hour: 1000,
        referal_code: generate_invite_code(data.id.to_string()),
        referals: Vec::new(),
        level: 10
    };

    match &data.from_referal {
        // Извлечение значения referal_code
        Some(referal_code_from_data) => {
            // Поиск рефераловода
            match state.token_collection.find_one(doc! { "referal_code": referal_code_from_data }, None).await {
                Ok(Some(mut d)) => {
                    // Новый реферал, добавление его id
                    d.referals.push(data.id.to_string());
                    // Подготовка данных для обновления
                    let update_doc = doc! { "$set": { "referals": &d.referals } };
                    match state.token_collection.update_one(doc! { "referal_code": referal_code_from_data }, update_doc, None).await {
                        Ok(_) => {}
                        Err(_) => {
                            let error = ErrorResponse { error: "Failed to update document".to_string() };
                            return HttpResponse::InternalServerError().json(error);
                        }
                    }
                    // Реферал добавлен
                    
                    // Поиск данных рфераловода
                    // let mut data_collection_value = match state.token_collection.find_one(doc! { "_id": &d._id }, None).await {
                    //     Ok(Some(d)) => d,
                    //     Ok(None) => {
                    //         let error = ErrorResponse { error: "User not found".to_string() };
                    //         return HttpResponse::NotFound().json(error);
                    //     }
                    //     Err(_) => {
                    //         let error = ErrorResponse { error: "Database query failed".to_string() };
                    //         return HttpResponse::InternalServerError().json(error);
                    //     }
                    // };
                    
                    d.oxi_tokens_value += 25000;
                    token_data.oxi_tokens_value += 25000;

                    let update_doc = doc! { "$set": { "oxi_tokens_value": Bson::from(d.oxi_tokens_value as i64) } };
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
    HttpResponse::Ok().json(create_success_response("OK"))
}

fn create_error_response(message: &str) -> ErrorResponse {
    ErrorResponse { error: message.to_string() }
}

fn create_success_response(message: &str) -> SuccessResponse {
    SuccessResponse { msg: message.to_string() }
}

async fn get_data(
    guard: web::Data<Mutex<AppState>>,
    query: web::Json<QueryUserData>
) -> impl Responder {
    let id = query.id.to_string();
    let state = guard.lock().await;

    let data = match state.token_collection.find_one(doc! { "_id": &id }, None).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            return HttpResponse::NotFound().json(create_error_response("User not found"));
        }
        Err(e) => {
            println!("{}", e);
            return HttpResponse::InternalServerError().json(create_error_response("Database query failed"));
        }
    };

    let response = data.build_response(&state, &data);
    match response {
        Ok(main_resp) => HttpResponse::Ok().json(main_resp),
        Err(http_resp) => http_resp,
    }
}

#[derive(Debug, Deserialize)]
struct ClaimTokensQuery {
    id: u64,
}

async fn claim_tokens(
    guard: web::Data<Mutex<AppState>>, 
    query: web::Json<ClaimTokensQuery>
) -> impl Responder {
    let user_id = query.id.clone().to_string();
    let state = guard.lock().await;

    let mut data = match state.token_collection.find_one(doc! { "_id": &user_id }, None).await {
        Ok(Some(d)) => d,
        Ok(None) => return HttpResponse::NotFound().json(ErrorResponse { error: "User not found".to_string() }),
        Err(_) => return HttpResponse::InternalServerError().json(ErrorResponse { error: "Database query failed".to_string() }),
    };

    let added_tokens = match state.update_tokens_value_vault(&user_id).await {
        Ok(tokens) => tokens,
        Err(_) => return HttpResponse::InternalServerError().json(ErrorResponse { error: "Failed to update token values".to_string() }),
    };

    data.oxi_tokens_value += added_tokens as u64;
    data.level += 50;

    let last_time_update = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs_f64(),
        Err(_) => {
            return HttpResponse::InternalServerError().json(ErrorResponse { error: "Failed to get current time".to_string() });
        }
    };
    
    data.last_time_update = last_time_update;

    match state.token_collection.replace_one(doc! { "_id": &user_id }, &data, None).await {
        Ok(_) => {},
        Err(_) => return HttpResponse::InternalServerError().json(ErrorResponse { error: "Failed to replace data in database".to_string() }),
    };

    let response = data.build_response(&state, &data);

    match response {
        Ok(main_resp) => HttpResponse::Ok().json(main_resp),
        Err(http_resp) => http_resp,
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct UpdateData {
    _id: u64,
    type_update: String,
    id_update: String
}

async fn update(
    state: web::Data<Mutex<AppState>>,
    data: web::Json<UpdateData>,
) -> impl Responder {
    let state = state.lock().await;
    let user_id = data._id.to_string();

    let mut token_data = match state.token_collection.find_one(doc! { "_id": &user_id }, None).await {
        Ok(Some(data)) => data,
        Ok(None) => {
            let error = ErrorResponse { error: "User not found".to_string() };
            return HttpResponse::NotFound().json(error);
        }
        Err(_) => {
            let error = ErrorResponse { error: "Database query failed".to_string() };
            return HttpResponse::InternalServerError().json(error);
        }
    };

    let current_level_upgrade = match token_data.upgrades.get(&data.id_update) {
        Some(level) => level,
        None => {
            token_data.upgrades.insert(data.id_update.to_string(), 0);
            &0
            // let error = ErrorResponse { error: "Upgrade not found for the user".to_string() };
            // return HttpResponse::BadRequest().json(error);
        }
    };

    let new_level_upgrade = current_level_upgrade + 1;
    if data.type_update == "miner" {
        if new_level_upgrade == 51 {
            let error = ErrorResponse { error: "Max level".to_string() };
            return HttpResponse::Ok().json(error);
        }
    };

    let new_level_data = if &data.type_update == "miner" {
        Some(state.upgrades_constant.miner.get(&((new_level_upgrade).to_string())).unwrap())
    } else {
        None
    };

    let current_level_data = if &data.type_update == "miner" {
        Some(state.upgrades_constant.miner.get(&((current_level_upgrade).to_string())).unwrap())
    } else {
        None
    };

    // Check if the user has enough tokens to perform the upgrade
    let new_level_data = match new_level_data {
        Some(data) => data,
        None => {
            let error = ErrorResponse { error: "Failed to get upgrade data".to_string() };
            return HttpResponse::InternalServerError().json(error);
        }
    };

    let current_level_data = match current_level_data {
        Some(data) => data,
        None => {
            let error = ErrorResponse { error: "Failed to get upgrade data".to_string() };
            return HttpResponse::InternalServerError().json(error);
        }
    };

    let mut tokens_add = current_level_data.tokens_add;
    let mut new_tokens_add = new_level_data.tokens_add;
    let mut buy_price = new_level_data.buy_price;


    if data.id_update == "miner_2" {
        tokens_add += tokens_add * 2 / 10;
        new_tokens_add += new_tokens_add * 2 / 10;
        buy_price += buy_price * 2 / 10;
    } else if data.id_update == "miner_3" {
        tokens_add += tokens_add * 3 / 10;
        new_tokens_add += new_tokens_add * 3 / 10;
        buy_price += buy_price * 3 / 10;
    } else if data.id_update == "miner_4" {
        tokens_add += tokens_add * 4 / 10;
        new_tokens_add += new_tokens_add * 4 / 10;
        buy_price += buy_price * 4 / 10;
    }

    if token_data.oxi_tokens_value < new_level_data.buy_price {
        let error = ErrorResponse { error: "Insufficient balance".to_string() };
        return HttpResponse::BadRequest().json(error);
    }

    // Perform the upgrade
    token_data.oxi_tokens_value -= buy_price;
    token_data.level += 100;
    token_data.tokens_hour += new_tokens_add - tokens_add;
    token_data.upgrades.insert(data.id_update.to_string(), new_level_upgrade);

    match state.token_collection.replace_one(doc! { "_id": &user_id }, &token_data, None).await {
        Ok(_) => {}
        Err(_) => {
            let error = ErrorResponse { error: "Failed to update user data".to_string() };
            return HttpResponse::InternalServerError().json(error);
        }
    }

    let response = token_data.build_response(&state, &token_data);
    match response {
        Ok(main_resp) => HttpResponse::Ok().json(main_resp),
        Err(http_resp) => http_resp,
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct MinerConfig {
    buy_price: u64,
    tokens_add: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    miner: HashMap<String, MinerConfig>,
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

    let upgrades_constant = match load_config("config/config.json") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error loading config: {}", e);
            panic!("Fail load config");
        } 
    };

    let password = "123";

    let state = web::Data::new(Mutex::new(AppState { 
        token_collection, 
        upgrades_constant,
        password: password.to_string()
    }));

    HttpServer::new(move || {
        App::new()
            .wrap(RateLimiter::new(100, Duration::from_secs(60))) // 100 запросов в минуту
            .app_data(state.clone())
            .route("/", web::get().to(index))
            .route("/api/data", web::post().to(get_data))
            .route("/api/update", web::post().to(update))
            .route("/claim_tokens", web::post().to(claim_tokens))
            .route("/newaccount", web::post().to(create_new_account))
            .service(actix_files::Files::new("/static", "./static").show_files_listing())
    })
    .bind(("127.0.0.1", 8081))?
    .run()
    .await
}