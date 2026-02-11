# Blackout

Uma ferramenta de gerenciamento de senhas segura e minimalista, construída em Rust com criptografia end-to-end e arquitetura daemon.

## 🔐 Características

- **Criptografia Forte**: Usa XChaCha20Poly1305 (AEAD de 256-bit) para proteção de dados
- **Key Derivation**: Argon2id com parâmetros configuráveis para derivar chaves a partir de senhas
- **Arquitetura Moderna**: Separação entre daemon de armazenamento e CLI client
- **Segurança em Memória**: Zeroize automático de dados sensíveis após uso
- **Persistência Segura**: Salt e nonce armazenados com dados criptografados para suportar rotação de chaves
- **Sem Dependências Desnecessárias**: Minimalista, sem frameworks pesados

## 🏗️ Arquitetura

```
blackout/
├── blackout-core/      # Lógica de criptografia, storage e vault
│   ├── vault.rs        # Estrutura Vault e KDF Argon2id
│   ├── storage.rs      # Persistência criptografada (XChaCha20Poly1305)
│   ├── event.rs        # Eventos do sistema
│   └── lib.rs          # Interface pública
├── blackout-daemon/    # Serviço background para gerenciar vault
│   └── main.rs         # Inicialização e gerenciamento de estado
└── blackoutctl/        # CLI para interação com daemon
    └── main.rs         # Interface de linha de comando
```

### Fluxo de Dados

1. **Escrita**: Password → Argon2id KDF → Derived Key (32 bytes) → XChaCha20Poly1305 encrypt → Persiste {salt, nonce, ciphertext}
2. **Leitura**: Load {salt, nonce, ciphertext} → Argon2id KDF com salt → Derived Key → XChaCha20Poly1305 decrypt → Desserializa Vault

## 🚀 Quick Start

### Compilar

```bash
cargo build --release
```

## 🔑 Segurança

### Criptografia

- **AEAD**: XChaCha20Poly1305 — encriptação autenticada com chave de 256-bit
- **KDF**: Argon2id — resistente a ataques GPU/ASIC (time: 2, memory: 19MB, parallelism: 1)
- **Nonce Management**: Nonce aleatório de 24 bytes (XChaCha20), único por mensagem, armazenado junto ao ciphertext
- **Memory Safety**: Zeroize chaves derivadas e plaintext em memória após uso

### Modelo de Ameaça

Protege contra:
- ✅ Acesso não autorizado ao arquivo vault.blackout
- ✅ Ataques de força bruta na senha (Argon2id com custo computacional alto)
- ✅ Disclosure de memória (zeroize automático)

**Não protege contra**:
- ❌ Exploração do daemon em execução (se comprometido, dados desencriptados em memória)
- ❌ Ataques side-channel (tempo, potência)
- ❌ Malware no sistema

## 📦 Dependências

- `argon2`: KDF Argon2id
- `chacha20poly1305`: Criptografia AEAD
- `serde_cbor`: Serialização de dados
- `chrono`: Timestamps
- `uuid`: IDs únicos
- `zeroize`: Limpeza de memória
- `rand`: Geração de números aleatórios
- `dirs`: Diretórios padrão do SO

## 🛠️ Desenvolvimento

### Requisitos

- Rust 1.70+
- Cargo

### Build Debug

```bash
cargo build
```


## 📋 Roadmap

- [ ] Tests
- [ ] Rotação de chaves
- [ ] Interface interativa para CLI
- [ ] Sincronização entre dispositivos
- [ ] Suporte a backup encriptado

## 📝 Convenções de Commit

Este projeto usa [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` nova feature
- `fix:` correção de bug
- `docs:` apenas documentação
- `style:` formatação, linting
- `refactor:` refatoração sem mudança de feature
- `perf:` otimizações
- `test:` testes
- `chore:` dependências, build, etc.

## 📄 Licença

MIT

## 👤 Autor

serafim

## 💡 Inspiração

Este projeto foi inspirado pelo [cipher0](https://github.com/batterdaysahead/cipher0) de [@batterdaysahead](https://github.com/batterdaysahead). Descobri o projeto logo após começar o desenvolvimento do Blackout e suas ideias e implementação influenciaram significativamente a arquitetura de segurança e criptografia aqui presente.

---

**⚠️ Disclaimer**: Esta é uma ferramenta educacional/experimental.
