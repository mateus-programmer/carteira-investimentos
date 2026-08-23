use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
    Router,
};
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::Cookie;
use askama::Template;
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::env;
use rust_decimal::Decimal;
use rust_decimal::prelude::Zero;

const SECRET_KEY: &[u8] = b"sua_chave_secreta_super_segura_aqui";

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

#[derive(sqlx::FromRow, Clone)]
struct Criptomoeda {
    id: i32,
    nome: String,
    simbolo: String,
    quantidade: Decimal,
    preco_medio: Decimal,
}

impl Criptomoeda {
    pub fn valor_total(&self) -> Decimal {
        self.quantidade * self.preco_medio
    }

    pub fn preco_medio_fmt(&self) -> String {
        format!("{:.2}", self.preco_medio)
    }

    pub fn valor_total_fmt(&self) -> String {
        format!("{:.2}", self.valor_total())
    }
}

#[derive(sqlx::FromRow)]
struct Usuario {
    id: i32,
    email: String,
    senha_hash: String,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    criptos: Vec<Criptomoeda>,
    patrimonio_total_fmt: String,
}

#[derive(Template)]
#[template(path = "registro.html")]
struct RegistroTemplate;

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate;

#[derive(Template)]
#[template(path = "nova_cripto.html")]
struct NovaCriptoTemplate;

#[derive(Deserialize)]
struct AuthForm {
    email: String,
    senha: String,
}

#[derive(Deserialize)]
struct CriptoForm {
    nome: String,
    simbolo: String,
    quantidade: Decimal,
    preco_medio: Decimal,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL deve ser definida no .env");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Falha ao conectar ao PostgreSQL");

    println!("Conectado ao banco de dados com sucesso!");

    let app = Router::new()
        .route("/", get(listar_criptos))
        .route("/registro", get(exibir_registro).post(cadastrar_usuario))
        .route("/login", get(exibir_login).post(autenticar_usuario))
        .route("/cripto/nova", get(exibir_nova_cripto).post(cadastrar_cripto))
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("Servidor rodando com sucesso em http://127.0.0.1:3000");

    axum::serve(listener, app).await.unwrap();
}

fn verificar_autenticacao(jar: &CookieJar) -> bool {
    let token = match jar.get("token") {
        Some(cookie) => cookie.value().to_string(),
        None => return false,
    };

    let validation = Validation::default();
    let decoding_key = DecodingKey::from_secret(SECRET_KEY);
    
    decode::<Claims>(&token, &decoding_key, &validation).is_ok()
}

async fn listar_criptos(
    State(pool): State<PgPool>,
    jar: CookieJar,
) -> impl IntoResponse {
    if !verificar_autenticacao(&jar) {
        return Redirect::to("/login").into_response();
    }

    let criptos = match sqlx::query_as::<_, Criptomoeda>(
        "SELECT id, nome, simbolo, quantidade, preco_medio FROM criptomoedas ORDER BY id ASC"
    )
    .fetch_all(&pool)
    .await {
        Ok(lista) => lista,
        Err(e) => {
            eprintln!("Erro ao buscar criptomoedas: {}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let soma_total: Decimal = criptos.iter().map(|c| c.valor_total()).sum();
    let patrimonio_total_fmt = format!("{:.2}", soma_total);

    let template = IndexTemplate {
        criptos,
        patrimonio_total_fmt,
    };
    
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            eprintln!("Erro ao renderizar template: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn exibir_registro() -> impl IntoResponse {
    let template = RegistroTemplate;
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            eprintln!("Erro ao renderizar template de registro: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn cadastrar_usuario(
    State(pool): State<PgPool>,
    Form(form): Form<AuthForm>,
) -> impl IntoResponse {
    let senha_hash = match bcrypt::hash(&form.senha, bcrypt::DEFAULT_COST) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Erro ao gerar hash da senha: {}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let resultado = sqlx::query(
        "INSERT INTO usuarios (email, senha_hash) VALUES ($1, $2)"
    )
    .bind(&form.email)
    .bind(&senha_hash)
    .execute(&pool)
    .await;

    match resultado {
        Ok(_) => {
            println!("Usuário cadastrado com sucesso: {}", form.email);
            Redirect::to("/login").into_response()
        }
        Err(e) => {
            eprintln!("Erro ao cadastrar usuário no banco: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn exibir_login() -> impl IntoResponse {
    let template = LoginTemplate;
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            eprintln!("Erro ao renderizar template de login: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn autenticar_usuario(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Form(form): Form<AuthForm>,
) -> impl IntoResponse {
    let usuario = match sqlx::query_as::<_, Usuario>(
        "SELECT id, email, senha_hash FROM usuarios WHERE email = $1"
    )
    .bind(&form.email)
    .fetch_optional(&pool)
    .await {
        Ok(Some(u)) => u,
        Ok(None) => {
            println!("Tentativa de login com e-mail não encontrado: {}", form.email);
            return StatusCode::UNAUTHORIZED.into_response();
        }
        Err(e) => {
            eprintln!("Erro ao buscar usuário no banco: {}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    match bcrypt::verify(&form.senha, &usuario.senha_hash) {
        Ok(true) => {
            println!("Login bem-sucedido para: {}", usuario.email);
            
            let expiration = chrono::Utc::now()
                .checked_add_signed(chrono::Duration::hours(24))
                .expect("tempo válido")
                .timestamp() as usize;

            let claims = Claims {
                sub: usuario.email,
                exp: expiration,
            };

            let token = encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret(SECRET_KEY),
            ).unwrap();

            let cookie = Cookie::build(("token", token))
                .path("/")
                .http_only(true)
                .build();

            (jar.add(cookie), Redirect::to("/")).into_response()
        }
        Ok(false) => {
            println!("Senha incorreta para o e-mail: {}", form.email);
            StatusCode::UNAUTHORIZED.into_response()
        }
        Err(e) => {
            eprintln!("Erro ao verificar senha: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn exibir_nova_cripto(jar: CookieJar) -> impl IntoResponse {
    if !verificar_autenticacao(&jar) {
        return Redirect::to("/login").into_response();
    }

    let template = NovaCriptoTemplate;
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            eprintln!("Erro ao renderizar template de nova cripto: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn cadastrar_cripto(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Form(form): Form<CriptoForm>,
) -> impl IntoResponse {
    if !verificar_autenticacao(&jar) {
        return Redirect::to("/login").into_response();
    }

    let simbolo_upper = form.simbolo.trim().to_uppercase();

    let existente = sqlx::query_as::<_, Criptomoeda>(
        "SELECT id, nome, simbolo, quantidade, preco_medio FROM criptomoedas WHERE UPPER(simbolo) = $1"
    )
    .bind(&simbolo_upper)
    .fetch_optional(&pool)
    .await;

    let resultado = match existente {
        Ok(Some(ativo)) => {
            let qtd_antiga = ativo.quantidade;
            let preco_antigo = ativo.preco_medio;
            
            let qtd_nova = form.quantidade;
            let preco_novo = form.preco_medio;

            let qtd_total = qtd_antiga + qtd_nova;
            
            let novo_preco_medio = if !qtd_total.is_zero() {
                ((qtd_antiga * preco_antigo) + (qtd_nova * preco_novo)) / qtd_total
            } else {
                Decimal::zero()
            };

            sqlx::query(
                "UPDATE criptomoedas SET quantidade = $1, preco_medio = $2 WHERE id = $3"
            )
            .bind(qtd_total)
            .bind(novo_preco_medio)
            .bind(ativo.id)
            .execute(&pool)
            .await
        }
        Ok(None) => {
            sqlx::query(
                "INSERT INTO criptomoedas (nome, simbolo, quantidade, preco_medio) VALUES ($1, $2, $3, $4)"
            )
            .bind(&form.nome)
            .bind(&simbolo_upper)
            .bind(form.quantidade)
            .bind(form.preco_medio)
            .execute(&pool)
            .await
        }
        Err(e) => {
            eprintln!("Erro ao consultar banco de dados: {}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    match resultado {
        Ok(_) => {
            println!("Ativo consolidado/cadastrado com sucesso: {}", simbolo_upper);
            Redirect::to("/").into_response()
        }
        Err(e) => {
            eprintln!("Erro ao salvar criptomoeda no banco: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}