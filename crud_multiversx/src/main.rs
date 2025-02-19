use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use hyper::{Body, Request, Response, Server, Method, StatusCode};
use hyper::service::{make_service_fn, service_fn};

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
struct Person {
    id: u64,
    name: String,
    age: u32,
}

type Db = Arc<Mutex<HashMap<u64, Person>>>;

async fn router(req: Request<Body>, db: Db) -> Result<Response<Body>, Infallible> {
    let path = req.uri().path().to_string();
    let method = req.method().clone();

    match (method, path.as_str()) {
        (Method::POST, "/persons") => {
            let whole_body = hyper::body::to_bytes(req.into_body()).await.unwrap();
            let mut new_person: Person = match serde_json::from_slice(&whole_body) {
                Ok(p) => p,
                Err(_) => {
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Body::from("JSON inválido"))
                        .unwrap());
                }
            };

            let mut db_lock = db.lock().unwrap();
            let new_id = if db_lock.is_empty() {
                1
            } else {
                db_lock.keys().max().unwrap() + 1
            };
            new_person.id = new_id;
            db_lock.insert(new_id, new_person.clone());

            let json = serde_json::to_string(&new_person).unwrap();
            Ok(Response::new(Body::from(json)))
        },
        (Method::GET, "/persons") => {
            let db_lock = db.lock().unwrap();
            let persons: Vec<&Person> = db_lock.values().collect();
            let json = serde_json::to_string(&persons).unwrap();
            Ok(Response::new(Body::from(json)))
        },
        (Method::GET, path) if path.starts_with("/persons/") => {
            let id_str = path.trim_start_matches("/persons/");
            match id_str.parse::<u64>() {
                Ok(id) => {
                    let db_lock = db.lock().unwrap();
                    if let Some(person) = db_lock.get(&id) {
                        let json = serde_json::to_string(person).unwrap();
                        Ok(Response::new(Body::from(json)))
                    } else {
                        Ok(Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .body(Body::from("Pessoa não encontrada"))
                            .unwrap())
                    }
                },
                Err(_) => {
                    Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Body::from("ID inválido"))
                        .unwrap())
                }
            }
        },
        (Method::PUT, path) if path.starts_with("/persons/") => {
            let id_str = path.trim_start_matches("/persons/");
            match id_str.parse::<u64>() {
                Ok(id) => {
                    let whole_body = hyper::body::to_bytes(req.into_body()).await.unwrap();
                    let updated_data: Person = match serde_json::from_slice(&whole_body) {
                        Ok(p) => p,
                        Err(_) => {
                            return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from("JSON inválido"))
                                .unwrap());
                        }
                    };

                    let mut db_lock = db.lock().unwrap();
                    if let Some(person) = db_lock.get_mut(&id) {
                        person.name = updated_data.name;
                        person.age = updated_data.age;
                        let json = serde_json::to_string(person).unwrap();
                        Ok(Response::new(Body::from(json)))
                    } else {
                        Ok(Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .body(Body::from("Pessoa não encontrada"))
                            .unwrap())
                    }
                },
                Err(_) => {
                    Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Body::from("ID inválido"))
                        .unwrap())
                }
            }
        },
        (Method::DELETE, path) if path.starts_with("/persons/") => {
            let id_str = path.trim_start_matches("/persons/");
            match id_str.parse::<u64>() {
                Ok(id) => {
                    let mut db_lock = db.lock().unwrap();
                    if db_lock.remove(&id).is_some() {
                        Ok(Response::new(Body::from("Pessoa removida")))
                    } else {
                        Ok(Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .body(Body::from("Pessoa não encontrada"))
                            .unwrap())
                    }
                },
                Err(_) => {
                    Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Body::from("ID inválido"))
                        .unwrap())
                }
            }
        },
        _ => {
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Rota não encontrada"))
                .unwrap())
        }
    }
}

#[tokio::main]
async fn main() {
    let db: Db = Arc::new(Mutex::new(HashMap::new()));

    let make_svc = make_service_fn(|_conn| {
        let db = db.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                router(req, db.clone())
            }))
        }
    });

    let addr = ([127, 0, 0, 1], 3000).into();

    let server = Server::bind(&addr).serve(make_svc);

    println!("Servidor rodando em http://{}", addr);

    if let Err(e) = server.await {
        eprintln!("Erro no servidor: {}", e);
    }
}
