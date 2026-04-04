use blackout_core::ipc::{Request, Response};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

const SOCKET_PATH: &str = "/tmp/blackout.sock";

fn main() {
    println!("Testando Ping");
    send_command(Request::Ping);

    println!("\nTestando Unlock");
    send_command(Request::Unlock {
        master_password: "pass1234".to_string(),
    });

    println!("\nTestando Unlock");
    send_command(Request::Lock);

    println!("\nTestando add entry");
    send_command(Request::AddEntry {
        service: "google.com".to_string(),
        user: "teste@gmail.com".to_string(),
        password: "senha_123".to_string()
    });

    println!("\nTestando add entry");
    send_command(Request::AddEntry {
        service: "google.com".to_string(),
        user: "teste@gmail.com".to_string(),
        password: "senha_123".to_string()
    });

    println!("\nTestando add entry");
    send_command(Request::AddEntry {
        service: "google.com".to_string(),
        user: "teste@gmail.com".to_string(),
        password: "senha_123".to_string()
    });

    println!("Testando List Entries");
    send_command(Request::ListEntries);

    println!("\nTestando Get Entry");
    send_command(Request::GetEntry {
        service: "google.com".to_string(),
    });
}

fn send_command(req: Request) {

    let mut stream = match UnixStream::connect(SOCKET_PATH) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Erro: Não foi possível conectar ao daemon. Ele está rodando?");
            return;
        }
    };

    let req_json = serde_json::to_string(&req).unwrap() + "\n";
    stream.write_all(req_json.as_bytes()).unwrap();

    let mut reader = BufReader::new(stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line).unwrap();

    match serde_json::from_str::<Response>(&response_line) {
        Ok(Response::Ok(msg)) => println!("Sucesso: {}", msg),
        Ok(Response::Error(err)) => {
            println!("Erro: {}", err);
            if err.contains("Vault is locked") {
                prompt_password_and_retry(req);
            }
        },
        Err(_) => eprintln!("Erro: Resposta malformada do daemon."),
    }
}

fn prompt_password_and_retry(req: Request) {
    use std::io::{self, Write};

    print!("Digite a senha mestre para desbloquear o vault: ");
    io::stdout().flush().unwrap();

    let mut password = String::new();
    io::stdin().read_line(&mut password).unwrap();
    let password = password.trim().to_string();

    let unlock_req = Request::Unlock { master_password: password };
    send_command(unlock_req);
    // After unlocking, retry the original request
    send_command(req);
}