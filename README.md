# Blackout

Uma ferramenta de gerenciamento de senhas segura e minimalista, construída em **Rust** com criptografia *end-to-end* e arquitetura cliente-servidor (daemon nativo via systemd).

## Características Principais

  * **Criptografia Autenticada**: Utiliza **XChaCha20Poly1305** (AEAD de 256-bit) para garantir confidencialidade e integridade dos dados.
  * **Key Derivation Robusta**: Implementa **Argon2id** com parâmetros configuráveis, oferecendo resistência contra ataques de GPU/ASIC.
  * **Arquitetura Cliente-Servidor**: Separação rigorosa entre o daemon de armazenamento (`blackoutd`) e o cliente TUI (`blackout`) via sockets Unix.
  * **Segurança Avançada e Memória**:
      * Uso extensivo da crate `zeroize` para limpar chaves e buffers sensíveis imediatamente após o uso.
      * Suporte nativo à flag `--mlock` para impedir que o Sistema Operacional faça *swap* de dados sensíveis da memória RAM para o disco.
  * **Snapshots e Versionamento**: Histórico automático de alterações do cofre, protegendo os dados contra edições acidentais (com suporte configurável via `max_snapshots`).
  * **Rotação de Chaves**: Capacidade de alterar a sua *Master Password* nativamente, reencriptando todo o cofre e rotacionando salt/nonce de forma segura.
  * **Interface TUI Moderna e Reativa**:
      * Cliente CLI construído com `ratatui`.
      * Indicador de versão do cofre em tempo real.
      * Modais de confirmação de segurança (prevenindo exclusões por "dedos gordos").
      * Integração nativa com **Wayland** para cópia ao *clipboard* (com limpeza automática configurável).

## Estrutura do Projeto

O projeto é dividido em três componentes principais para garantir modularidade e segurança:

```text
blackout/
├── blackout-core/  # Biblioteca central: Criptografia, Storage (XChaCha20Poly1305) e Versionamento
├── blackoutd/      # Daemon: Serviço background gerido via systemd que mantém o estado e gerencia IPC
└── blackout/       # TUI: Interface interativa construida com Ratatui
```

### Fluxo de Criptografia

1.  **Derivação**: Password + Salt → **Argon2id** → 256-bit Key.
2.  **Proteção**: Vault + Key + Nonce → **XChaCha20Poly1305** → Encrypted Storage.

## Como usar

### Instalação

Certifique-se de ter o Rust (1.92+) instalado em um ambiente Linux com systemd.

```bash
# Clone o repositório
git clone https://github.com/Vinicin1101/blackout
cd blackout

# Dê permissão de execução ao script
chmod +x setup.sh

# Instalação Padrão (configura o serviço systemd automaticamente)
./setup.sh install

# Instalação com proteção de despejo (Recomendado)
# Impede que o Daemon faça swap de memória para o disco
./setup.sh install --mlock
```

### Execução e Configuração

A instalação configura o `blackoutd` para rodar automaticamente em background como um serviço de usuário (`systemctl --user`). Você não precisa iniciá-lo manualmente.

Para gerenciar suas senhas, basta chamar o cliente TUI de qualquer terminal:

```bash
blackout
```

**Personalização (Opcional):**
O Blackout suporta um arquivo `config.toml` (em `~/.config/blackout/` ou equivalente) onde você pode definir:

  * `auto_lock_timeout`: Tempo de inatividade para bloquear o cofre automaticamente.
  * `max_snapshots`: Quantidade de backups mantidos do cofre (Poda estrita).

### Desinstalação

Para remover os binários e desativar o serviço:

```bash
./setup.sh uninstall
# Nota: O comando padrão mantém o seu cofre salvo. 
# Para destruir o cofre e os dados permanentemente, use: ./setup.sh uninstall --purge
```

## Atalhos da Interface

[![Built With Ratatui](https://img.shields.io/badge/Built_With_Ratatui-000?logo=ratatui&logoColor=fff)](https://ratatui.rs/)

| Tecla | Ação |
| :--- | :--- |
| `Enter` | Ver detalhes da entrada / Submeter formulário / Copiar campo para área de transferêcia (wayland) |
| `n` | Criar nova entrada |
| `e` | Editar a entrada selecionada |
| `F2` | Alternar visibilidade da senha (mostrar/ocultar) |
| `Backspace` | Excluir entrada selecionada |
| `x` | Bloquear cofre imediatamente |
| `Esc` | Voltar / Cancelar ação / Sair |
| `Tab` / `B-Tab` | Navegar entre campos dos formulários |

## Modelo de Ameaça

### O que o Blackout protege:

  * **Acesso ao disco**: Atacantes com o arquivo do cofre (ou snapshots) não podem ler os dados sem a master password.
  * **Força Bruta**: O custo do `Argon2id` retarda significativamente tentativas de cracking.
  * **Memory Dumps**: O `zeroize` (aliado à inicialização `--mlock`) minimiza severamente a chance de dados sensíveis serem recuperados da memória física ou de paginação.
  * **Leak de Clipboard**: A área de transferência é apagada automaticamente.

### O que o Blackout NÃO protege:

  * **Comprometimento do Daemon**: Se o daemon for explorado por outro processo no mesmo usuário enquanto o cofre estiver aberto na memória.
  * **Keyloggers**: Captura de teclas no nível do Sistema Operacional.

## Roadmap

  - [x] Interface TUI com Tabelas e Layout Centralizado
  - [x] Renomeação para padrão Daemon Unix (`blackoutd`) com systemd
  - [x] CRUD Completo: Edição de entradas
  - [x] Rotação de chaves e troca de Master Password
  - [x] Criação automática de Snapshots / Versionamento de Cofre
  - [x] Restauração de Snapshots (Rollback) via UI
  - [ ] Exportação segura do cofre via QR Code
  - [ ] CLI (Acesso direto não-interativo para scripts)
  - [ ] Testes de Unidade e Integração

## Licença

Distribuído sob a licença **MIT**. Veja `LICENSE` para mais informações.

-----

**AVISO**: Esta é uma ferramenta experimental desenvolvida para fins educacionais. Use com cautela e por sua conta e risco. (Recomenda-se manter backups independentes de suas credenciais importantes).
