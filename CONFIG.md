# Guia de Configuração do Blackout (`config.toml`)

O arquivo de configuração do Blackout reside por padrão em `~/.local/share/blackout/config.toml` e atua como a **Fonte Única da Verdade (SSOT)** para o comportamento do daemon (`blackoutd`) e do mecanismo de Autofill da interface.

---

## Parâmetros Globais do Core e Daemon

### `auto_lock_timeout`
* **Tipo:** Inteiro (Segundos)
* **Padrão:** `30`
* **Descrição:** Determina o tempo máximo de inatividade do usuário antes de o daemon limpar a chave mestre da memória RAM e bloquear o cofre novamente. 
* **Impacto:** Valores menores aumentam a segurança contra ataques físicos no terminal aberto. Definir como `0` desativa o bloqueio automático (não recomendado).

### `max_snapshots`
* **Tipo:** Inteiro
* **Padrão:** `0` (Ilimitado)
* **Descrição:** O número máximo de snapshots (histórico de backups `.bak`) que o daemon manterá armazenados em `~/.local/share/blackout/.snapshots/`.
* **Impacto:** Quando um novo limite positivo $Y$ é definido, a próxima modificação no cofre aplicará um **purge imediato**, eliminando de uma vez todo o excesso de histórico antigo acumulado para liberar espaço em disco.

---

## Seção `[password_generation]`

Esta seção dita o comportamento padrão do motor do gerador de senhas contido na `blackout-core` e é consumido de forma estrita sempre que o atalho de **Autofill (`Ctrl+A`)** é acionado dentro de um formulário.

### `mode`
* **Tipo:** String (Enum)
* **Valores Aceitos:** `"RandomChars"` ou `"Passphrase"`
* **Descrição:** Seleciona o algoritmo base do gerador.
  * `"RandomChars"`: Gera cadeias de caracteres baseadas em entropia bruta.
  * `"Passphrase"`: Gera frases de segurança legíveis utilizando a wordlist de alta entropia da **EFF (Electronic Frontier Foundation)**.

### `length`
* **Tipo:** Inteiro (Quantidade de caracteres)
* **Intervalo Válido:** o quanto sua máquina suportar (futuramente será limitado)
* **Descrição:** Define o comprimento total do texto gerado quando o `mode` está configurado para `"RandomChars"`. É ignorado no modo `"Passphrase"`.

### `word_count`
* **Tipo:** Inteiro (Quantidade de palavras) 
* **Intervalo Válido:** o quanto sua máquina suportar (futuramente será limitado)
* **Descrição:** Define o número de palavras sorteadas do dicionário da EFF quando o `mode` está configurado para `"Passphrase"`. É ignorado no modo `"RandomChars"`.

### `separator`
* **Tipo:** String (Caractere único)
* **Padrão:** `"_"`
* **Descrição:** O caractere delimitador usado para unir as palavras sorteadas no modo `"Passphrase"`. Exemplos comuns: `"-"`, `"_"`, `"."`. É ignorado no modo `"RandomChars"`.

### Campos Booleanos de Regras de Charset

Os parâmetros abaixo são do tipo **Booleano (`true` / `false`)** e controlam a composição da string final. 

| Campo | Modo Aplicado | Descrição |
| :--- | :--- | :--- |
| `capitalize` | `"Passphrase"` | Quando `true`, transforma a primeira letra de cada palavra sorteada em maiúscula (ex: `Gato_Azul_Forte`). |
| `uppercase` | `"RandomChars"` | Permite a inclusão de caracteres maiúsculos (`A-Z`) na geração aleatória. |
| `lowercase` | `"RandomChars"` | Permite a inclusão de caracteres minúsculos (`a-z`) na geração aleatória. |
| `numbers` | `"RandomChars"` | Permite a inclusão de dígitos numéricos (`0-9`) na geração aleatória. |
| `symbols` | `"RandomChars"` | Permite a inclusão de caracteres especiais e símbolos (ex: `"!" "@" "#" "$" "%" "*"`) na geração aleatória. |

---

## Exemplo de Arquivo Completo

```toml
# Configurações de Ciclo de Vida do Cofre
auto_lock_timeout = 45
max_snapshots = 10

# Configurações Padrão de Fábrica do Autofill (Ctrl+A)
[password_generation]
mode = "RandomChars"
length = 24
word_count = 4
separator = "-"
capitalize = true
uppercase = true
lowercase = true
numbers = true
symbols = true
