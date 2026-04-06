# Blackout

Uma ferramenta de gerenciamento de senhas segura e minimalista, construída em **Rust** com criptografia *end-to-end* e arquitetura cliente-servidor (daemon).

## Características Principais

  * **Criptografia Autenticada**: Utiliza **XChaCha20Poly1305** (AEAD de 256-bit) para garantir confidencialidade e integridade dos dados.
  * **Key Derivation Robusta**: Implementa **Argon2id** com parâmetros configuráveis, oferecendo resistência contra ataques de GPU/ASIC.
  * **Arquitetura Cliente-Servidor**: Separação rigorosa entre o daemon de armazenamento (`blackoutd`) e o cliente de linha de comando (`blackout`) via sockets Unix.
  * **Interface TUI Moderna**: Cliente CLI construído com `ratatui`, apresentando tabelas formatadas, layout centralizado e visualização de metadados (como `updated_at`).
  * **Segurança em Memória**: Uso extensivo da crate `zeroize` para limpar chaves e buffers sensíveis imediatamente após o uso.
  * **Persistência Segura**: Salt e nonce aleatórios armazenados junto ao *ciphertext*, suportando futuras rotações de chaves.

## Estrutura do Projeto

O projeto é dividido em três componentes principais para garantir modularidade e segurança:

```text
blackout/
├── blackout-core/  # Biblioteca central: Criptografia, Storage (XChaCha20Poly1305) e Vault
├── blackoutd/     # Daemon: Serviço background que mantém o estado do cofre e gerencia IPC
└── blackout/   # TUI: Interface interativa para o usuário final
```

### Fluxo de Criptografia

1.  **Derivação**: Password + Salt → **Argon2id** → 256-bit Key.
2.  **Proteção**: Vault + Key + Nonce → **XChaCha20Poly1305** → Encrypted Storage.

## Como começar

### Instalação

```bash
# Clone o repositório
git clone https://github.com/Vinicin1101/blackout
cd blackout

# Dê permissão de execução ao script
chmod +x setup.sh

# Instale o Blackout para o seu usuário
./setup.sh install
```

## Demo

![](https://i.imgur.com/9AbkTcy.gif)

## Atalhos da Interface (`blackout`)

[![Built With Ratatui](https://img.shields.io/badge/Built_With_Ratatui-000?logo=ratatui&logoColor=fff)](https://ratatui.rs/)

| Tecla | Ação |
| :--- | :--- |
| `Enter` | Ver detalhes da entrada / Submeter formulário |
| `n` | Criar nova entrada |
| `Backspace` | Excluir entrada selecionada |
| `x` | Bloquear cofre |
| `Esc` | Voltar / Sair |
| `Tab` / `B-Tab` | Navegar entre campos do formulário |

## Modelo de Ameaça

### O que o Blackout protege:

  * ✅ **Acesso ao disco**: Atacantes com o arquivo do cofre não podem ler os dados sem a master password.
  * ✅ **Força Bruta**: O custo do `Argon2id` retarda significativamente tentativas de cracking.
  * ✅ **Memory Dumps**: O `zeroize` minimiza o tempo que dados sensíveis residem na RAM.
  * ✅ **Leak**: Seus dados com você.

### O que o Blackout NÃO protege:

  * ❌ **Comprometimento do Daemon**: Se o daemon for explorado enquanto o cofre estiver aberto, as senhas em cache podem ser expostas.
  * ❌ **Keyloggers**: Captura de teclas no nível do Sistema Operacional.

## Roadmap

  - [x] Interface TUI com Tabelas e Layout Centralizado
  - [x] Renomeação para padrão Daemon Unix (`blackoutd`)
  - [ ] Edição de entradas
  - [ ] CLI
  - [ ] Testes de Unidade e Integração
  - [ ] Rotação de chaves e troca de Master Password
  - [ ] Sincronização e Backups encriptados

## Licença

Distribuído sob a licença **MIT**. Veja `LICENSE` para mais informações.

-----

**AVISO**: Esta é uma ferramenta experimental desenvolvida para fins educacionais. Use com cautela e por sua conta e risco.
