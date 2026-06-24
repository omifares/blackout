# Blackout

Uma ferramenta de gerenciamento de senhas segura e minimalista, construída em **Rust** com criptografia *end-to-end* e arquitetura cliente-servidor (daemon nativo via systemd).

## Características Principais

  * **Criptografia Autenticada**: Utiliza **XChaCha20Poly1305** (AEAD de 256-bit).
  * **Key Derivation Robusta**: **Argon2id** com parâmetros configuráveis.
  * **Gerador de Senhas Integrado**: Suporte para geração de senhas seguras (caracteres aleatórios e passphrases) com autofill integrado.
  * **Arquitetura Cliente-Servidor**: Daemon (`blackoutd`) + Cliente TUI (`blackout`).
  * **Segurança Avançada**: Uso de `zeroize` para limpeza de memória e suporte a `--mlock`.
  * **Snapshot e Versão**: Histórico automático de cofre.
  * **Interface TUI Moderna**: Cliente CLI construído com `ratatui`.

## Estrutura do Projeto

```text
blackout/
├── blackout-core/  # Biblioteca central (Criptografia e lógica de geração)
├── blackoutd/      # Daemon: Serviço background (IPC)
└── blackout/    # TUI: Interface interativa (Publicado como 'blackout-ui')
```

### Fluxo de Criptografia

1.  **Derivação**: Password + Salt → **Argon2id** → 256-bit Key.
2.  **Proteção**: Vault + Key + Nonce → **XChaCha20Poly1305** → Encrypted Storage.

---

## Instalação

### Build from source (recomendado)

Certifique-se de ter o Rust (1.92+) instalado em um ambiente Linux com systemd.

```bash
# Clone o repositório
git clone https://github.com/omifares/blackout
cd blackout

# Dê permissão de execução ao script
chmod +x setup.sh

# Instalação Padrão (configura o serviço systemd automaticamente)
./setup.sh install

# Instalação com proteção de despejo (Recomendado)
# Impede que o Daemon faça swap de memória para o disco
./setup.sh install --mlock
```

### Cargo
```bash
# TUI
cargo install blackout-ui

# Daemon
cargo install blackoutd
```
> Nota: A instalação via cargo fornece apenas o binário do cliente (blackout). Ela não configura o daemon (blackoutd) nem cria o arquivo de serviço blackoutd.service do systemd.

## Desinstalação

Para remover os binários e desativar o serviço:

```bash
./setup.sh uninstall
# Nota: O comando padrão mantém o seu cofre salvo. 
# Para destruir o cofre e os dados permanentemente, use: ./setup.sh uninstall --purge
```

---

## Execução e Configuração

A instalação atraveś do `setup.sh` configura o `blackoutd` para rodar automaticamente em background como um serviço de usuário (`systemctl --user`). Você não precisa iniciá-lo manualmente.

Para gerenciar suas senhas, basta chamar o cliente TUI de qualquer terminal:

```bash
blackout
```

### Gerenciamento de Estado e Configurações (config.toml vs Sessão)

O Blackout adota uma arquitetura transacional rígida e unidirecional para proteger a integridade das suas preferências no disco:

- O Arquivo é a Fonte da Verdade: O arquivo `config.toml` (`XDG_DATA_HOME/blackout` ou `~/.local/share/blackout`) nunca é alterado ou reescrito automaticamente pela interface (UI). A interface apenas lê e reflete o que está escrito nele.

- Isolamento em Sandbox (Sessão Volátil): Qualquer alteração feita nas definições do Gerador de Senhas através dos menus da interface altera apenas o estado do programa em memória para a sessão atual. Assim que o programa é finalizado ou fechado, essas alterações são descartadas e o Blackout carregará o config.toml limpo na próxima execução.

- Comportamento do Autofill: Para garantir consistência e segurança contra configurações acidentais em tempo de execução, o mecanismo de Autofill (Ctrl+A) respeita estritamente as regras do arquivo config.toml, e não as modificações temporárias feitas na sessão da interface.

### Modos de Geração Suportados no config.toml

O motor do gerador contido no blackout-core possui dois modos distintos configuráveis ():

- Password (Random Chars): Geração baseada em entropia pura combinando caracteres alfanuméricos e símbolos especiais com comprimentos customizáveis.
- Passphrase (Wordlist EFF): Geração de frases de segurança baseadas em dicionários de alta entropia da EFF (Electronic Frontier Foundation), ideais para senhas mestras ou credenciais que exigem memorização sem perda de força criptográfica.

### Veja mais informações de configurações [CONFIG](CONFIG.md)

---

## Atalhos da Interface

[![Built With Ratatui](https://img.shields.io/badge/Built_With_Ratatui-000?logo=ratatui&logoColor=fff)](https://ratatui.rs/)

| Tecla | Ação | Contexto |
| :--- | :--- | :--- |
| `Ctrl + A` | Gerar e preencher senha (Autofill) | Formulário de Senha |
| `Enter` | Ver detalhes / Submeter / Copiar | Geral |
| `n` | Criar nova entrada | Listagem |
| `e` | Editar a entrada selecionada | Listagem |
| `F2` | Alternar visibilidade da senha,Formulários |
| `Setas` | Navegação,Geral |
| `x` | Bloquear cofre | Geral |
| `Esc` | Voltar / Cancelar | Geral |
| `Tab` | Navegar entre campos | Formulários |

---

## Modelo de Ameaça

### O que o Blackout protege:

  * **Acesso ao disco**: Atacantes com o arquivo do cofre (ou snapshots) não podem ler os dados sem a master password.
  * **Força Bruta**: O custo do `Argon2id` retarda significativamente tentativas de cracking.
  * **Memory Dumps**: O `zeroize` (aliado à inicialização `--mlock`) minimiza severamente a chance de dados sensíveis serem recuperados da memória física ou de paginação.
  * **Leak de Clipboard**: A área de transferência é apagada automaticamente.

### O que o Blackout NÃO protege:

  * **Comprometimento do Daemon**: Se o daemon for explorado por outro processo no mesmo usuário enquanto o cofre estiver aberto na memória.
  * **Keyloggers**: Captura de teclas no nível do Sistema Operacional.

---

## Roadmap

  - [x] Interface TUI com Tabelas e Layout Centralizado
  - [x] Renomeação para padrão Daemon Unix (`blackoutd`) com systemd
  - [x] CRUD Completo: Edição de entradas
  - [x] Rotação de chaves e troca de Master Password
  - [x] Criação automática de Snapshots / Versionamento de Cofre
  - [x] Restauração de Snapshots (Rollback) via UI
  - [x] Built-in Pass Generator
  - [ ] Exportação segura do cofre via QR Code
  - [ ] CLI (Acesso direto não-interativo para scripts)
  - [ ] Testes de Unidade e Integração

---

## Licença

Distribuído sob a licença **MIT**. Veja `LICENSE` para mais informações.

-----

**AVISO**: Esta é uma ferramenta experimental desenvolvida para fins educacionais. Use com cautela e por sua conta e risco. (Recomenda-se manter backups independentes de suas credenciais importantes).
