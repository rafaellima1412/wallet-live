# wallet-live

Aplicação web para controle de carteira de investimentos, construída em Rust com Axum, SQLx (PostgreSQL) e Askama.

> Documentação em construção — este README cobre por enquanto o setup do banco de dados, como rodar a aplicação e como rodar os testes. Uma versão mais completa (arquitetura, rotas, exemplos de uso) está a caminho.

## Stack

- **Axum** — servidor web e roteamento
- **SQLx** — acesso ao PostgreSQL com queries verificadas em tempo de compilação
- **Askama** — templates HTML
- **JWT + cookies** — autenticação
- **CoinGecko API** — atualização automática das cotações dos ativos
- **Docker Compose** — banco de dados PostgreSQL

## Pré-requisitos

- Rust (edition 2024 — use uma toolchain recente via https://rustup.rs)
- Docker e Docker Compose

## Como rodar

1. Suba o banco de dados:

   ```bash
   docker compose up -d
   ```

2. Copie o arquivo de variáveis de ambiente:

   ```bash
   cp .env.example .env
   ```

3. Rode a aplicação:

   ```bash
   cargo run
   ```

   As migrations são aplicadas automaticamente na inicialização. O servidor sobe em `http://localhost:3000`.

4. Acesse `http://localhost:3000`.

   O login também funciona como cadastro: se o usuário não existir, ele é criado na primeira tentativa de login.

## Atualização automática de cotações

Cada ativo pode possuir um campo opcional `coingecko_id`, que representa o identificador utilizado pela API da CoinGecko (por exemplo, `bitcoin` ou `ethereum`).

Quando esse campo estiver preenchido, o dashboard disponibiliza o botão **"Atualizar cotações"**, que consulta os preços atuais na CoinGecko e atualiza automaticamente o campo `unit_value` no banco de dados.

Ativos sem `coingecko_id` continuam funcionando normalmente, mantendo o valor informado manualmente.

### Exemplo

Para consultar o preço do Bitcoin em dólares:

```http
POST /assets/refresh-prices
```

Parâmetros:

```text
ids=bitcoin
vs_currencies=usd
```

A CoinGecko utiliza **IDs**, e não tickers. Portanto:

| Correto (`coingecko_id`) | Incorreto |
|--------------------------|-----------|
| bitcoin                  | BTC       |
| ethereum                 | ETH       |
| solana                   | SOL       |

A lista completa de identificadores pode ser consultada em:

https://api.coingecko.com/api/v3/coins/list

## Testes

Os testes de integração usam o macro `sqlx::test`, que cria um banco de dados isolado para cada teste (aplicando as migrations automaticamente) e o descarta ao final. Por isso, é necessário ter o Postgres do `docker compose` rodando e a variável `DATABASE_URL` configurada:

```bash
docker compose up -d
cargo test
```

Cobertura atual:

- **`src/repository.rs`** — criação, listagem e atualização de ativos; cadastro e busca de usuários (incluindo username duplicado); registro e listagem de ativos possuídos (cálculo de quantidade e variação de valor).
- **`src/auth/user.rs`** — registro de usuário, autenticação com senha correta/incorreta, usuário inexistente e round-trip do token JWT (geração e validação).

## Estrutura do projeto

```
src/
  app.rs           # bootstrap da aplicação (estado, conexão com o banco, migrations)
  auth/            # autenticação (usuário via JWT/cookie, admin via header)
  router/
    api.rs         # rotas JSON
    frontend.rs    # rotas HTML
  repository.rs    # acesso a dados (SQLx)
  model.rs         # structs de domínio
  error.rs         # erros da aplicação
  quote.rs     # integração com a API da CoinGecko
migrations/        # migrations do SQLx
templates/         # templates Askama
```

## Limitações atuais / próximos passos

- A chave do JWT (`SECRET_KEY`) ainda está fixa no código-fonte e deve migrar para uma variável de ambiente.
- O cookie de autenticação ainda não está assinado nem marcado como `Secure`.
- Os testes permanecem próximos aos módulos (`#[cfg(test)]`). Como a aplicação atualmente é um único binário, separar testes unitários e de integração em uma estrutura mais elaborada ainda não traz benefícios significativos. Caso o projeto evolua para múltiplos serviços ou crates, a estratégia de testes será reorganizada para refletir essa separação.

## Cadastro de ativos

- Atualmente, o cadastro de novos ativos está disponível apenas pela API REST (POST /api/assets), destinada a operações administrativas.

- A interface web permite registrar compras e atualizar cotações, mas ainda não possui um formulário para criação de novos ativos. A inclusão desse formulário no frontend está prevista para versões futuras

## API REST — Cadastro e gerenciamento de ativos

O cadastro de ativos está disponível apenas pela API REST. Abaixo estão os principais endpoints relacionados:
```
GET /swagger-ui
GET /api-docs/openapi.json
```