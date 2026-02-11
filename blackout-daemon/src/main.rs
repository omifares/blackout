use blackout_core::vault::Vault;
use blackout_core::storage::Wallet;

fn main() {
    println!("Blackout Daemon ativo...");

    // Inicializa o storage
    let storage = Wallet::init();
    
    // Tenta carregar o vault existente ou cria um novo
    let mut current_vault = if storage.exists() {
        Vault::default() 
    } else {
        Vault::default()
    };

    // Adição de uma nova entrada
    current_vault.add_entry("example.com".to_string(), "macacos_me_mordam".to_string(), "civicuda123".to_string());

    // Salva o vault atualizado com senha
    let password = "my_super_secret_password";
    storage.encrypt_and_save_vault(&current_vault, password).expect("Falha ao salvar o vault");

    // Exemplo de leitura do vault (com a mesma senha)
    let loaded_vault = storage.load_vault(password).expect("Falha ao carregar o vault");
    println!("Vault carregado: {:?}", loaded_vault);

    // Exemplo com senha errada
    match storage.load_vault("wrong_password") {
        Ok(_) => println!("Isso não deveria acontecer!"),
        Err(e) => println!("Erro esperado ao carregar com senha errada: {}", e),
    }
}
