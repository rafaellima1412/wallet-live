# wallet-live

Aplicação web para controle de carteira de investimentos, construída em Rust com Axum, SQLx (PostgreSQL) e Askama.

> Documentação em construção — este README cobre por enquanto o setup do banco de dados, como rodar a aplicação e como rodar os testes. Uma versão mais completa (arquitetura, rotas, exemplos de uso) está a caminho.

## Stack

- **Axum** — servidor web e roteamento
- **SQLx** — acesso ao PostgreSQL com queries verificadas em tempo de compilação
- **Askama** — templates HTML
- **JWT + cookies** — autenticação
- **Docker Compose** — banco de dados PostgreSQL

## Pré-requisitos

- Rust (edition 2024 — use uma toolchain recente via [rustup](https://rustup.rs))
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

4. Acesse `http://localhost:3000` — o login também funciona como cadastro: se o usuário não existir, ele é criado na primeira tentativa de login.

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
  app.rs          # bootstrap da aplicação (estado, conexão com o banco, migrations)
  auth/           # autenticação (usuário via JWT/cookie, admin via header)
  router/
    api.rs        # rotas JSON (/api/assets)
    frontend.rs    # rotas HTML (login, dashboard de ativos)
  model.rs        # structs de domínio
  repository.rs   # acesso a dados (SQLx)
  error.rs        # erros da aplicação e mapeamento para respostas HTTP
migrations/       # migrations do SQLx
templates/        # templates Askama
```

## Segurança / próximos passos conhecidos

- As chaves secretas (`SECRET_KEY` do JWT e `ADMIN_SECRET_KEY`) ainda estão fixas no código-fonte; devem migrar para variáveis de ambiente.
- O cookie de autenticação ainda não está assinado nem marcado como `secure`.

Cada ativo ganha um campo opcional coingecko_id (ex: bitcoin, ethereum) — é o identificador que a CoinGecko usa, não o ticker.
Um botão novo no dashboard, "atualizar cotações", busca o preço atual de todos os ativos que tiverem esse campo preenchido e atualiza o unit_value no banco.
Ativos sem coingecko_id continuam funcionando manualmente, como hoje.

    /// Busca o preço atual de cada moeda em `ids` na moeda `vs_currency` (ex: "usd").
    /// `ids` deve usar o identificador da CoinGecko (ex: "bitcoin", "ethereum"),
    /// e não o ticker (ex: "BTC", "ETH"). A lista completa de ids está em
    /// https://api.coingecko.com/api/v3/coins/list