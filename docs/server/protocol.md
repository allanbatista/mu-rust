# Protocolo Client/Server (Rust)

Atualizado em: 2026-02-08

## Objetivo
Registrar o estado real da integração entre HTTP login, sessão autenticada e protocolo QUIC no `rust/server`, usando `cpp/OpenMU` apenas como referência funcional.

## Fase 1 (Concluída): Login HTTP -> Hello QUIC autenticado

### Implementado
- `POST /login` agora retorna `auth_token` assinado (HMAC-SHA256) junto com `account_id`.
- Token de autenticação carrega:
  - `account_id`
  - `session_id` HTTP
  - `expires_at_ms`
  - lista de personagens autorizados para a sessão
- `ClientMessage::Hello` é obrigatório antes de qualquer ação de gameplay.
- `ServerMessage::HelloAck` agora inclui `characters` (lista de personagens autorizados).
- Pacotes sem sessão autenticada retornam `ServerErrorKind::InvalidSession`.
- `SelectCharacter` só aceita personagem pertencente ao token autenticado.

### Contrato impactado
- `LoginResponse` inclui `auth_token`.
- `ServerMessage::HelloAck` inclui `characters: Vec<CharacterSummary>`.

## Fase 2 (Concluída): Transfer seguro e vínculo com sessão HTTP

### Implementado
- `ClientHello` valida token **e** sessão HTTP real (`SessionManager`) quando o runtime está ligado ao servidor HTTP.
- `MapTransferDirective.route_token` passou a ser token assinado com TTL (não é mais string fixa).
- `ClientMessage::MapTransferAck` agora exige:
  - `transfer_id`
  - `route_token`
- `MapTransferAck` valida:
  - assinatura/expiração do `route_token`
  - `session_id` QUIC
  - `transfer_id`
  - `character_id`
  - `route`
- Bloqueio de personagem ativo em múltiplas sessões QUIC:
  - personagem em uso por outra sessão é rejeitado com `InvalidAction`.
- Logout/expiração limpa sessão de mapa e transfers pendentes.

### Contrato impactado
- `ClientMessage::MapTransferAck` mudou para `MapTransferAck { transfer_id, route_token }`.

## Fase 3 (Concluída): Fluxo e2e de validação operacional

### Implementado
- `sim_client` atualizado para fluxo completo:
  1. `Hello`
  2. lê `HelloAck` e escolhe personagem
  3. `SelectCharacter`
  4. recebe `MapTransfer`
  5. `MapTransferAck` com `route_token`
  6. recebe `EnterMap`
  7. envia `Move` em datagrama
  8. valida retorno `StateDelta`
- Endpoint `GET /characters` agora inclui `protocol_character_id` para alinhar API HTTP e IDs usados no protocolo QUIC.
- Teste adicional de segurança: `MapTransferAck` com `route_token` inválido é rejeitado.

## Validação executada
- `cargo fmt --all`
- `cargo check --workspace`
- `cargo test --workspace`

Todos os testes do workspace passaram após as mudanças.

## Próximas fases (Backlog)
- Broadcast de `StateDelta` server-driven por tick (não apenas resposta direta ao `Move`).
- Chat/party/guild/global com fan-out via `MessageHub`.
- Combate e skill pipeline autoritativo (damage, hit validation, deaths, drops).
- Economia transacional (trade/inventory/zen) com persistência forte e idempotência.
- Reconexão de sessão QUIC e retomada de personagem/mapa.
- Integração de cliente Bevy em tempo real com este protocolo (atualmente o `sim_client` é o verificador e2e).
