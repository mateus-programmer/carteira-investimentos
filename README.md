# 🚀 Carteira Digital de Investimentos em Rust

Aplicação Fullstack desenvolvida em **Rust** com o objetivo de demonstrar segurança, tipagem forte e alto desempenho no desenvolvimento web moderno. O projeto conecta Back-End, banco de dados relacional, segurança avançada e interface em Server-Side Rendering (SSR).

## 🛠️ Tecnologias Utilizadas

* **Servidor & API:** Axum (Ecossistema Tokio).
* **Banco de Dados:** PostgreSQL gerenciado via SQLx com suporte a Type-Safe SQL.
* **Frontend (SSR):** Askama (Template engine type-safe).
* **Autenticação:** Stateless com JWT (JSON Web Tokens) e persistência de sessões via Cookies HTTP-only.
* **Segurança:** Hash de senhas com bcrypt e precisão financeira com rust_decimal.

## ✨ Funcionalidades

* Cadastro e autenticação segura de usuários.
* Dashboard principal exibindo a carteira de criptoativos do usuário.
* Cálculo automático do Valor Total por ativo e do Patrimônio Total da Carteira.
* Lógica inteligente de consolidação de ativos (cálculo de preço médio ponderado ao adicionar novos aportes).