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

    let login_res = auth.login(login_req).await.unwrap();

    assert!(!login_res.user_id.is_empty());
    assert!(!login_res.access_token.is_empty());
    assert!(!login_res.refresh_token.is_empty());
    assert_eq!(login_res.device_id, device_name);

    let (user_id, device_id) = auth
        .verify_token(&login_res.access_token)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(user_id, login_res.user_id);
    assert_eq!(device_id, device_name);

    // Clean up redis
    let _: () = redis::cmd("DEL")
        .arg(format!("token:{}", login_res.access_token))
        .query_async(&mut conn)
        .await
        .unwrap();
    let _: () = redis::cmd("DEL")
        .arg(format!("token:{}", login_res.refresh_token))
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

    let login_res = auth.login(login_req).await.unwrap();

    let refresh_res = auth.refresh_token(&login_res.refresh_token).await.unwrap();

    assert!(!refresh_res.access_token.is_empty());

    let (user_id, device_id) = auth
        .verify_token(&refresh_res.access_token)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(user_id, login_res.user_id);
    assert_eq!(device_id, device_name);

    // Clean up redis
    let _: () = redis::cmd("DEL")
        .arg(format!("token:{}", login_res.access_token))
        .query_async(&mut conn)
        .await
        .unwrap();
    let _: () = redis::cmd("DEL")
        .arg(format!("token:{}", login_res.refresh_token))
        .query_async(&mut conn)
        .await
        .unwrap();
    let _: () = redis::cmd("DEL")
        .arg(format!("token:{}", refresh_res.access_token))
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

    let login_res = auth.login(login_req).await.unwrap();

    auth.logout(&login_res.access_token, &login_res.refresh_token)
        .await
        .unwrap();

    let access_token_verification = auth.verify_token(&login_res.access_token).await.unwrap();
    assert!(access_token_verification.is_none());

    let refresh_token_verification = auth.verify_token(&login_res.refresh_token).await.unwrap();
    assert!(refresh_token_verification.is_none());
}
