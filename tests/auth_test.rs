use eko_messenger::auth::{Auth, LoginRequest};
use eko_messenger::firebase_auth::FirebaseAuth;
use redis::aio::MultiplexedConnection;
use std::env;

async fn setup() -> (Auth<FirebaseAuth>, MultiplexedConnection) {
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set");
    let client = redis::Client::open(redis_url).unwrap();
    let conn = client.get_multiplexed_async_connection().await.unwrap();
    let firebase_auth = FirebaseAuth::new_from_env().unwrap();
    let auth = Auth::new(firebase_auth, conn.clone());
    (auth, conn)
}

#[tokio::test]
async fn test_login_and_verify_token() {
    if env::var("FIREBASE_API_KEY").is_err() {
        println!("Skipping test: FIREBASE_API_KEY not set");
        return;
    }
    let email = match env::var("TEST_USER_EMAIL") {
        Ok(email) => email,
        Err(_) => {
            println!("Skipping test: TEST_USER_EMAIL not set");
            return;
        }
    };
    let password = match env::var("TEST_USER_PASSWORD") {
        Ok(password) => password,
        Err(_) => {
            println!("Skipping test: TEST_USER_PASSWORD not set");
            return;
        }
    };

    let (auth, mut conn) = setup().await;

    let device_name = "test_device".to_string();

    let login_req = LoginRequest {
        email: email.clone(),
        password,
        device_name: device_name.clone(),
    };

    let login_res = auth
        .login(login_req, "127.0.0.1", "test-agent")
        .await
        .unwrap();

    assert!(!login_res.access_token.is_empty());
    assert!(!login_res.refresh_token.is_empty());

    let claims = auth
        .verify_access_token(&login_res.access_token)
        .unwrap();

    assert!(!claims.sub.is_empty());

    // Clean up redis
    let _: () = redis::cmd("DEL")
        .arg(format!("rt:{}", login_res.refresh_token))
        .query_async(&mut conn)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_refresh_token() {
    if env::var("FIREBASE_API_KEY").is_err() {
        println!("Skipping test: FIREBASE_API_KEY not set");
        return;
    }
    let email = match env::var("TEST_USER_EMAIL") {
        Ok(email) => email,
        Err(_) => {
            println!("Skipping test: TEST_USER_EMAIL not set");
            return;
        }
    };
    let password = match env::var("TEST_USER_PASSWORD") {
        Ok(password) => password,
        Err(_) => {
            println!("Skipping test: TEST_USER_PASSWORD not set");
            return;
        }
    };

    let (auth, mut conn) = setup().await;
    let device_name = "test_device".to_string();

    let login_req = LoginRequest {
        email: email.clone(),
        password,
        device_name: device_name.clone(),
    };

    let login_res = auth
        .login(login_req, "127.0.0.1", "test-agent")
        .await
        .unwrap();

    println!(
        "[test_refresh_token] Refresh token from login: {}",
        login_res.refresh_token
    );

    let redis_key = format!("rt:{}", login_res.refresh_token);
    let value: Option<String> = redis::cmd("HGET")
        .arg(&redis_key)
        .arg("userId")
        .query_async(&mut conn)
        .await
        .unwrap();
    println!(
        "[test_refresh_token] Value from redis for key {}: {:?}",
        redis_key, value
    );

    let refresh_res = auth
        .refresh_token(&login_res.refresh_token, "127.0.0.1", "test-agent")
        .await
        .unwrap();

    assert!(!refresh_res.access_token.is_empty());

    let claims = auth
        .verify_access_token(&refresh_res.access_token)
        .unwrap();

    let first_claims = auth
        .verify_access_token(&login_res.access_token)
        .unwrap();
    assert_eq!(claims.sub, first_claims.sub);

    // Clean up redis
    let _: () = redis::cmd("DEL")
        .arg(format!("rt:{}", refresh_res.refresh_token))
        .query_async(&mut conn)
        .await
        .unwrap();
    let _: () = redis::cmd("DEL")
        .arg(format!("user_rt:{}", claims.sub))
        .query_async(&mut conn)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_logout() {
    if env::var("FIREBASE_API_KEY").is_err() {
        println!("Skipping test: FIREBASE_API_KEY not set");
        return;
    }
    let email = match env::var("TEST_USER_EMAIL") {
        Ok(email) => email,
        Err(_) => {
            println!("Skipping test: TEST_USER_EMAIL not set");
            return;
        }
    };
    let password = match env::var("TEST_USER_PASSWORD") {
        Ok(password) => password,
        Err(_) => {
            println!("Skipping test: TEST_USER_PASSWORD not set");
            return;
        }
    };

    let (auth, _) = setup().await;
    let device_name = "test_device".to_string();

    let login_req = LoginRequest {
        email: email.clone(),
        password,
        device_name: device_name.clone(),
    };

    let login_res = auth
        .login(login_req, "127.0.0.1", "test-agent")
        .await
        .unwrap();

    let claims = auth
        .verify_access_token(&login_res.access_token)
        .unwrap();
    let user_id = claims.sub;

    auth.logout(&login_res.refresh_token, &user_id)
        .await
        .unwrap();

    let refresh_result = auth
        .refresh_token(&login_res.refresh_token, "127.0.0.1", "test-agent")
        .await;
    assert!(refresh_result.is_err());
}
