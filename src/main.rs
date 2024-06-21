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
    username: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
    register_in_game: f64,
    upgrades: HashMap<String, u8>,
    language: String,
    oxi_tokens_value: u64,
    last_time_update: f64,
    tokens_hour: u64,
    referal_code: String,
    referals: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct MainResponse {
    _id: String,
    username: Option<String>,
    upgrades_current: Option<HashMap<String, HashMap<String, String>>>,
    upgrades_new: Option<HashMap<String, HashMap<String, String>>>,
    oxi_tokens_value: u64,
    last_time_update: f64,
    tokens_hour: u64,
    referal_code: String,
    referals: Vec<String>,
    referals_value: String,
}

impl TokenData {
    fn build_response(&self, upgrades_chapshot: Option<HashMap<String, HashMap<String, String>>>, upgrades_chapshot_new: Option<HashMap<String, HashMap<String, String>>>) -> MainResponse {
        MainResponse {
            _id: self._id.clone(),
            username: self.username.clone(),
            upgrades_current: upgrades_chapshot,
            upgrades_new: upgrades_chapshot_new,
            oxi_tokens_value: self.oxi_tokens_value,
            last_time_update: self.last_time_update,
            tokens_hour: self.tokens_hour,
            referal_code: self.referal_code.clone(),
            referals: self.referals.clone(),
            referals_value: self.referals.len().to_string()
        }
    }
}

struct AppState {
    token_collection: mongodb::Collection<TokenData>,
    vault_size_constant: HashMap<u8, u32>,
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
        let filter = doc! { "_id": id };
  
        let data_result = self.token_collection.find_one(filter.clone(), None).await;
        let data = match data_result {
            Ok(Some(doc)) => doc,
            Ok(None) => return Err(UpdateError::NotFound),
            Err(_) => return Err(UpdateError::DatabaseError),
        };
        
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs_f64();
        let time_difference = current_time - data.last_time_update;
        let time_difference_in_hours = time_difference / 3600.0;
        let added_tokens = (time_difference_in_hours * 1000.0) as u64;
        let vault_size = self.vault_size_constant[&data.upgrades.get("vault_main").unwrap()] as u64;
    
        if added_tokens > vault_size {
            return Ok(vault_size);
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
        return HttpResponse::BadRequest().json(create_error_response("Auth error"));
    }

    if let Err(response) = check_user_registration(&state, &data.id.to_string()).await {
        return response;
    }

    let last_time_update = match get_current_time() {
        Ok(time) => time,
        Err(response) => return response,
    };

    let mut token_data = create_token_data(&data, last_time_update);

    if let Some(response) = process_referal(&state, &data, &mut token_data).await {
        return response;
    }

    if let Err(response) = insert_token_data(&state, token_data).await {
        return response;
    }

    HttpResponse::Ok().json(create_success_response("User registered"))
}

fn create_error_response(message: &str) -> ErrorResponse {
    ErrorResponse { error: message.to_string() }
}

fn create_success_response(message: &str) -> SuccessResponse {
    SuccessResponse { msg: message.to_string() }
}

async fn check_user_registration(state: &AppState, user_id: &String) -> Result<(), HttpResponse> {
    match state.token_collection.count_documents(doc! {"_id": user_id.to_string()}, None).await {
        Ok(count) if count > 0 => Err(HttpResponse::BadRequest().json(create_error_response("User already register"))),
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Failed to count documents: {:?}", e);
            Err(HttpResponse::InternalServerError().json(create_error_response("Internal Server Error")))
        }
    }
}

fn get_current_time() -> Result<f64, HttpResponse> {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => Ok(duration.as_secs_f64()),
        Err(_) => Err(HttpResponse::InternalServerError().json(create_error_response("Failed to get current time"))),
    }
}

fn create_token_data(data: &RequestRegister, last_time_update: f64) -> TokenData {
    let upgrades = HashMap::from([
        ("miner_1".to_string(), 1),
        ("vault_main".to_string(), 1),
    ]);

    TokenData {
        _id: data.id.to_string(),
        username: data.username.clone(),
        first_name: data.first_name.clone(),
        last_name: data.last_name.clone(),
        register_in_game: last_time_update,
        upgrades,
        language: data.language.clone(),
        oxi_tokens_value: 0,
        last_time_update,
        tokens_hour: 1000,
        referal_code: generate_invite_code(data.id.to_string()),
        referals: Vec::new(),
    }
}

async fn process_referal(state: &AppState, data: &RequestRegister, token_data: &mut TokenData) -> Option<HttpResponse> {
    if let Some(referal_code) = &data.from_referal {
        match state.token_collection.find_one(doc! { "referal_code": referal_code }, None).await {
            Ok(Some(mut referal_user)) => {
                referal_user.referals.push(data.id.to_string());
                if let Err(_) = update_referal_user(&state, &referal_user, referal_code).await {
                    return Some(HttpResponse::InternalServerError().json(create_error_response("Failed to update document")));
                }
                if let Err(_) = update_token_value(&state, &referal_user).await {
                    return Some(HttpResponse::InternalServerError().json(create_error_response("Failed to update token value")));
                }
                token_data.oxi_tokens_value += 25000;
            }
            Ok(None) => {}
            Err(_) => return Some(HttpResponse::InternalServerError().json(create_error_response("Database query failed"))),
        }
    }
    None
}

async fn update_referal_user(state: &AppState, referal_user: &TokenData, referal_code: &str) -> Result<(), HttpResponse> {
    let update_doc = doc! { "$set": { "referals": &referal_user.referals } };
    state.token_collection.update_one(doc! { "referal_code": referal_code }, update_doc, None).await
        .map_err(|_| HttpResponse::InternalServerError().json(create_error_response("Failed to update document")))?;
    Ok(())
}

async fn update_token_value(state: &AppState, referal_user: &TokenData) -> Result<(), HttpResponse> {
    let update_doc = doc! { "$set": { "oxi_tokens_value": Bson::from(referal_user.oxi_tokens_value as i64) } };
    state.token_collection.update_one(doc! { "_id": &referal_user._id }, update_doc, None).await
        .map_err(|err| {
            println!("{}", err);
            HttpResponse::InternalServerError().json(create_error_response("Failed to update token value"))
        })?;
    Ok(())
}

async fn insert_token_data(state: &AppState, token_data: TokenData) -> Result<(), HttpResponse> {
    state.token_collection.insert_one(token_data, None).await
        .map_err(|err| {
            println!("{}", err);
            HttpResponse::InternalServerError().json(create_error_response("Failed to insert data in database"))
        })?;
    Ok(())
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
        Err(_) => {
            return HttpResponse::InternalServerError().json(create_error_response("Database query failed"));
        }
    };

    let mut upgrades_chapshot = HashMap::new();
    let mut upgrades_chapshot_new = HashMap::new();

    for (key, b) in &data.upgrades {
        println!("{} {}", key, b);

        let mut upgrades_local = HashMap::new();
        let mut upgrades_new = HashMap::new();

        let parts: Vec<&str> = key.split('_').collect();
        println!("{}", parts[0]);

        if "miner" == parts[0] { 
            let current_level_upgrade = match state.upgrades_constant.miner.get(&b.to_string()) {
                Some(v) => v,
                None => {
                    let error = ErrorResponse { error: "Failed to Data Base".to_string() };
                    return HttpResponse::InternalServerError().json(error);
                }
            };

            let new_level_upgrade = match state.upgrades_constant.miner.get(&(b + 1).to_string()) {
                Some(v) => v,
                None => {
                    let error = ErrorResponse { error: "Failed to Data Base".to_string() };
                    return HttpResponse::InternalServerError().json(error);
                }
            };
            upgrades_local.insert("tokens_hour".to_string(), current_level_upgrade.tokens_add.to_string());
            upgrades_local.insert("level".to_string(), b.to_string());

            upgrades_new.insert("tokens_hour".to_string(), new_level_upgrade.tokens_add.to_string());
            upgrades_new.insert("price".to_string(), new_level_upgrade.buy_price.to_string());

            upgrades_chapshot.insert(key.to_string(), upgrades_local.clone());
            upgrades_chapshot_new.insert(key.to_string(), upgrades_new.clone());

        } else if parts[0] == "vault" {
            let current_level_upgrade = match state.upgrades_constant.vault.get(&b.to_string()) {
                Some(v) => v,
                None => {
                    let error = ErrorResponse { error: "Failed to Data Base".to_string() };
                    return HttpResponse::InternalServerError().json(error);
                }
            };

            let new_level_upgrade = match state.upgrades_constant.vault.get(&(b + 1).to_string()) {
                Some(v) => v,
                None => {
                    let error = ErrorResponse { error: "Failed to Data Base".to_string() };
                    return HttpResponse::InternalServerError().json(error);
                }
            };
            upgrades_local.insert("volume".to_string(), current_level_upgrade.volume.to_string());
            upgrades_local.insert("level".to_string(), b.to_string());

            upgrades_new.insert("volume".to_string(), new_level_upgrade.volume.to_string());

            upgrades_chapshot.insert(key.to_string(), upgrades_local.clone());
            upgrades_chapshot_new.insert(key.to_string(), upgrades_new.clone());
        }
    }

    println!("{:?}", upgrades_chapshot);
    println!("{:?}", upgrades_chapshot_new);

    let response = data.build_response(Some(upgrades_chapshot), Some(upgrades_chapshot_new));

    HttpResponse::Ok().json(response)
}

#[derive(Debug, Deserialize)]
struct ClaimTokensQuery {
    id: String,
}

async fn claim_tokens(
    guard: web::Data<Mutex<AppState>>, 
    query: web::Json<ClaimTokensQuery>
) -> impl Responder {
    let user_id = query.id.clone();
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

    let response = data.build_response(None, None);

    HttpResponse::Ok().json(response)
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
        Some(level) => *level,
        None => {
            let error = ErrorResponse { error: "Upgrade not found for the user".to_string() };
            return HttpResponse::BadRequest().json(error);
        }
    };

    let new_level_upgrade = current_level_upgrade + 1;
    let new_level_data = if &data.type_update == "miner" {
        Some(state.upgrades_constant.miner.get(&((new_level_upgrade).to_string())).unwrap())
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

    if token_data.oxi_tokens_value < new_level_data.buy_price {
        let error = ErrorResponse { error: "Insufficient balance".to_string() };
        return HttpResponse::BadRequest().json(error);
    }

    // Perform the upgrade
    token_data.oxi_tokens_value -= new_level_data.buy_price;
    token_data.tokens_hour += new_level_data.tokens_add;

    match state.token_collection.replace_one(doc! { "_id": &user_id }, &token_data, None).await {
        Ok(_) => {}
        Err(_) => {
            let error = ErrorResponse { error: "Failed to update user data".to_string() };
            return HttpResponse::InternalServerError().json(error);
        }
    }

    let mut upgrades_chapshot = HashMap::new();
    let mut upgrades_chapshot_new = HashMap::new();

    for (key, b) in &token_data.upgrades {
        println!("{} {}", key, b);

        let mut upgrades_local = HashMap::new();
        let mut upgrades_new = HashMap::new();

        let parts: Vec<&str> = key.split('_').collect();
        println!("{}", parts[0]);

        if "miner" == parts[0] { 
            let current_level_upgrade = match state.upgrades_constant.miner.get(&b.to_string()) {
                Some(v) => v,
                None => {
                    let error = ErrorResponse { error: "Failed to Data Base".to_string() };
                    return HttpResponse::InternalServerError().json(error);
                }
            };

            let new_level_upgrade = match state.upgrades_constant.miner.get(&(b + 1).to_string()) {
                Some(v) => v,
                None => {
                    let error = ErrorResponse { error: "Failed to Data Base".to_string() };
                    return HttpResponse::InternalServerError().json(error);
                }
            };
            upgrades_local.insert("tokens_hour".to_string(), current_level_upgrade.tokens_add.to_string());
            upgrades_local.insert("level".to_string(), b.to_string());

            upgrades_new.insert("tokens_hour".to_string(), new_level_upgrade.tokens_add.to_string());
            upgrades_new.insert("price".to_string(), new_level_upgrade.buy_price.to_string());

            upgrades_chapshot.insert(key.to_string(), upgrades_local.clone());
            upgrades_chapshot_new.insert(key.to_string(), upgrades_new.clone());

        } else if parts[0] == "vault" {
            let current_level_upgrade = match state.upgrades_constant.vault.get(&b.to_string()) {
                Some(v) => v,
                None => {
                    let error = ErrorResponse { error: "Failed to Data Base".to_string() };
                    return HttpResponse::InternalServerError().json(error);
                }
            };

            let new_level_upgrade = match state.upgrades_constant.vault.get(&(b + 1).to_string()) {
                Some(v) => v,
                None => {
                    let error = ErrorResponse { error: "Failed to Data Base".to_string() };
                    return HttpResponse::InternalServerError().json(error);
                }
            };
            upgrades_local.insert("volume".to_string(), current_level_upgrade.volume.to_string());
            upgrades_local.insert("level".to_string(), b.to_string());

            upgrades_new.insert("volume".to_string(), new_level_upgrade.volume.to_string());

            upgrades_chapshot.insert(key.to_string(), upgrades_local.clone());
            upgrades_chapshot_new.insert(key.to_string(), upgrades_new.clone());
        }
    }

    println!("{:?}", upgrades_chapshot);
    println!("{:?}", upgrades_chapshot_new);

    let response = token_data.build_response(Some(upgrades_chapshot), Some(upgrades_chapshot_new));

    HttpResponse::Ok().json(response)
}

#[derive(Serialize, Deserialize, Debug)]
struct MinerConfig {
    buy_price: u64,
    tokens_add: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct VaultConfig {
    volume: u32
}

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    vault: HashMap<String, VaultConfig>,
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

    let password = "123";

    let state = web::Data::new(Mutex::new(AppState { 
        token_collection, 
        vault_size_constant,
        upgrades_constant,
        password: password.to_string()
    }));

    HttpServer::new(move || {
        App::new()
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