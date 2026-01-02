use eko_messenger::auth::{FirebaseAuth, IdentityProvider};
use std::env;
use tokio;

#[tokio::test]
async fn test_firebase_login_with_email() {
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

    let firebase_auth = FirebaseAuth::new_from_env_with_domain("https://localhost:3000".to_string()).await.unwrap();

    let result = firebase_auth.login_with_email(email, password).await;

    assert!(result.is_ok(), "Login failed: {:?}", result.err());
    let (person, uid) = result.unwrap();
    assert!(!uid.is_empty());
    assert_eq!(person.type_field, "Person");
}

