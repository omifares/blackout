use blackout_core::ipc::{Request, Response};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

const SOCKET_PATH: &str = "/tmp/blackout.sock";

fn main() {
    // Teste 1: Ping
    println!("Testando Ping");
    send_command(Request::Ping);

    // Teste 2: Unlock
    println!("\nTestando Unlock");
    send_command(Request::Unlock {
        master_password: "pass1234".to_string(),
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
        Ok(Response::Error(err)) => eprintln!("Erro do Daemon: {}", err),
        Err(_) => eprintln!("Erro: Resposta malformada do daemon."),
    }
}
