//! Generates a PASETO v4.public key pair for development.
//! Usage: cargo run -p shared --example gen_keys

fn main() {
    let (secret, public) = shared::paseto::generate_keypair().expect("keygen failed");
    println!("PASETO_SECRET_KEY={secret}");
    println!("PASETO_PUBLIC_KEY={public}");
    eprintln!(
        "DEVELOPMENT Keys. In production, the secret key comes from a secret manager and is distributed only to the Auth service."
    )
}
