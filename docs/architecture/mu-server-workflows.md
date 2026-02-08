# MU Server Workflows (10k CCU, single bare metal first)

## Escopo
Este documento descreve os principais casos de uso da arquitetura proposta para MU Online com foco em:
- ate 10k usuarios concorrentes
- capacidade de rodar em 1 servidor bare metal
- evolucao gradual para multi-host sem quebrar contratos internos
- prioridade de gameplay de characters sobre AI de monstros
- reducao de hit em banco via buffer + flush periodico

## Contexto de dominio
- Um `World` (ex.: Midgard) possui multiplos `EntryPoint` (ex.: Midgard-1, Midgard-2).
- Cada `EntryPoint` possui varios `MapServer` (1 processo/logica por mapa, com instancias quando necessario).
- O estado de gameplay e autoritativo no `MapServer`.
- Persistencia critica e sincrona; persistencia nao critica usa buffering e flush em lote.

## Componentes logicos
- `Gateway`: login, sessao, roteamento para world/entry.
- `WorldDirectory`: catalogo de worlds, entry points, lotacao e health.
- `MapServer`: simulacao de players e regras de mapa.
- `MonsterWorker` (opcional): processa AI pesada para mapas quentes.
- `MessageHub`: chat/eventos cross-map e cross-entry.
- `PersistenceWorker`: coalescing de estado nao critico + batch writes.
- `WAL`: journal local para eventos criticos.
- `Database`: armazenamento persistente (contas, chars, inventario, transacoes).

## Casos de uso e workflows

### UC-01 - Boot da stack em single bare metal
```mermaid
flowchart TD
    A[Start mu-core] --> B[Load config]
    B --> C[Init Gateway]
    C --> D[Init WorldDirectory]
    D --> E[Init MessageHub]
    E --> F[Init PersistenceWorker]
    F --> G[Init WAL]
    G --> H[Connect Database]
    H --> I{DB ok?}
    I -- nao --> J[Fail fast + retry policy]
    I -- sim --> K[Spawn worlds/entry points]
    K --> L[Spawn MapServers por mapa base]
    L --> M[Start health loops]
    M --> N[Server ready]
```

### UC-02 - Login com sessao unica por conta
```mermaid
sequenceDiagram
    autonumber
    actor Player
    participant Gateway
    participant Database
    participant SessionStore

    Player->>Gateway: POST /login (username, password)
    Gateway->>Database: validar credenciais
    Database-->>Gateway: ok / erro

    alt credencial invalida
        Gateway-->>Player: 401
    else credencial valida
        Gateway->>SessionStore: verificar sessao ativa da conta
        alt sessao antiga existe
            Gateway->>SessionStore: invalidar sessao antiga
            Gateway->>Player: evento "kicked" para sessao antiga
        end
        Gateway->>SessionStore: criar nova sessao
        Gateway-->>Player: 200 + session token
    end
```

### UC-03 - Descoberta de World e EntryPoint
```mermaid
sequenceDiagram
    autonumber
    actor Player
    participant Gateway
    participant WorldDirectory

    Player->>Gateway: GET /worlds
    Gateway->>WorldDirectory: listar worlds + entry points + lotacao
    WorldDirectory-->>Gateway: worlds online
    Gateway-->>Player: lista ordenada por disponibilidade
```

### UC-04 - Selecao de character e entrada no mapa inicial
```mermaid
sequenceDiagram
    autonumber
    actor Player
    participant Gateway
    participant Database
    participant WorldDirectory
    participant MapServer

    Player->>Gateway: POST /characters/select
    Gateway->>Database: validar ownership + estado do character
    Database-->>Gateway: validado
    Gateway->>WorldDirectory: escolher entry point e map target
    WorldDirectory-->>Gateway: route (world_id, entry_id, map_id, instance_id)
    Gateway->>MapServer: reserve slot + preload character
    MapServer-->>Gateway: route token
    Gateway-->>Player: endpoint + route token
    Player->>MapServer: connect(route token)
    MapServer-->>Player: spawn ack + snapshot inicial
```

### UC-05 - Tick de gameplay com prioridade de player
```mermaid
flowchart TD
    A[Tick start 50ms] --> B[Process input de players]
    B --> C[Resolver combate/skills de players]
    C --> D[Aplicar colisao e estado critico]
    D --> E[Broadcast updates para players]
    E --> F[Executar AI de monstros em budget]
    F --> G{CPU budget estourou?}
    G -- sim --> H[Degradar AI monstros: menos pathfinding e menor freq]
    G -- nao --> I[AI completa]
    H --> J[Marcar entidades dirty para persistencia]
    I --> J
    J --> K[Tick end]
```

### UC-06 - AI de monstros no mesmo processo (padrao)
```mermaid
sequenceDiagram
    autonumber
    participant MapServer
    participant MonsterSubsystem

    loop a cada tick de monstro (ex.: 100-200ms)
        MapServer->>MonsterSubsystem: snapshot leve do estado local
        MonsterSubsystem-->>MapServer: intents (move, attack, cast)
        MapServer->>MapServer: validar intents no estado autoritativo
        MapServer->>MapServer: aplicar intents validos
    end
```

### UC-07 - AI de monstros em worker separado (opcional)
```mermaid
sequenceDiagram
    autonumber
    participant MapServer
    participant MonsterWorker

    MapServer->>MonsterWorker: enviar snapshot delta (mapa quente)
    MonsterWorker-->>MapServer: intents de AI
    MapServer->>MapServer: validar intents

    alt worker atrasou ou falhou
        MapServer->>MapServer: fallback para AI simplificada local
    else worker saudavel
        MapServer->>MapServer: aplicar intents normais
    end
```

### UC-08 - Troca de mapa (intra-world)
```mermaid
sequenceDiagram
    autonumber
    actor Player
    participant SourceMap as MapServer A
    participant Gateway
    participant WorldDirectory
    participant TargetMap as MapServer B
    participant PersistenceWorker

    Player->>SourceMap: request map change (portal)
    SourceMap->>SourceMap: validar pre-condicoes
    SourceMap->>Gateway: request route target
    Gateway->>WorldDirectory: resolve map instance destino
    WorldDirectory-->>Gateway: map route
    Gateway->>TargetMap: reserve slot
    SourceMap->>PersistenceWorker: enqueue snapshot de transicao
    SourceMap-->>Player: transfer token
    Player->>TargetMap: connect transfer token
    TargetMap-->>Player: spawn em mapa destino
    SourceMap->>SourceMap: liberar entidades locais
```

### UC-09 - Chat local, party, guild e global
```mermaid
sequenceDiagram
    autonumber
    actor Player
    participant MapServer
    participant MessageHub
    participant RecvMap as MapServer Destino

    Player->>MapServer: chat message(channel, payload)
    MapServer->>MapServer: validar mute/rate limit/sanitizacao

    alt canal local
        MapServer-->>Player: echo local
        MapServer->>MapServer: broadcast no mapa atual
    else canal party/guild/global/whisper
        MapServer->>MessageHub: publish(channel_key, message)
        MessageHub->>RecvMap: fanout por assinantes
        RecvMap->>RecvMap: entregar para sessoes locais
    end
```

### UC-10 - Persistencia nao critica com buffer e flush
```mermaid
sequenceDiagram
    autonumber
    participant MapServer
    participant PersistenceWorker
    participant Database

    MapServer->>PersistenceWorker: upsert dirty state (char_id -> last_state)

    loop a cada flush_tick (ex.: 2s)
        PersistenceWorker->>PersistenceWorker: coalesce por char_id
        PersistenceWorker->>Database: bulk upsert em lote
        Database-->>PersistenceWorker: ack/erro
        alt erro transiente
            PersistenceWorker->>PersistenceWorker: retry com backoff
        end
    end
```

### UC-11 - Operacao critica (trade/inventario/zen/cash)
```mermaid
sequenceDiagram
    autonumber
    actor PlayerA
    actor PlayerB
    participant MapServer
    participant WAL
    participant Database

    PlayerA->>MapServer: confirmar trade
    PlayerB->>MapServer: confirmar trade
    MapServer->>MapServer: validar regras + lock de entidades
    MapServer->>WAL: append evento critico (pre-commit)
    WAL-->>MapServer: fsync ok
    MapServer->>Database: transacao atomica (debit/credit/item move)
    Database-->>MapServer: commit ok/erro

    alt commit ok
        MapServer-->>PlayerA: trade success
        MapServer-->>PlayerB: trade success
    else commit erro
        MapServer-->>PlayerA: trade failed
        MapServer-->>PlayerB: trade failed
    end
```

### UC-12 - Recuperacao apos crash (WAL replay)
```mermaid
flowchart TD
    A[Process restart] --> B[Load ultimo checkpoint]
    B --> C[Ler WAL nao confirmado]
    C --> D{Evento critico pendente?}
    D -- nao --> E[Start normal]
    D -- sim --> F[Reaplicar evento com idempotency key]
    F --> G{Reaplicacao ok?}
    G -- sim --> H[Marcar WAL como confirmado]
    G -- nao --> I[Marcar para reconciliacao/manual review]
    H --> D
    I --> D
```

### UC-13 - Logout e desconexao inesperada
```mermaid
sequenceDiagram
    autonumber
    actor Player
    participant Gateway
    participant MapServer
    participant PersistenceWorker
    participant SessionStore

    alt logout explicito
        Player->>Gateway: POST /logout
        Gateway->>MapServer: detach character
    else timeout/disconnect
        MapServer->>MapServer: detectar timeout
    end

    MapServer->>PersistenceWorker: flush imediato do estado final
    Gateway->>SessionStore: invalidar sessao
    Gateway-->>Player: logout ack (se online)
```

### UC-14 - Shutdown gracioso
```mermaid
flowchart TD
    A[Signal TERM] --> B[Stop accept de novas conexoes]
    B --> C[Notificar players: server closing]
    C --> D[Flush critico imediato]
    D --> E[Flush nao critico final]
    E --> F[Commit pendencias WAL]
    F --> G[Encerrar MapServers]
    G --> H[Encerrar Gateway e MessageHub]
    H --> I[Shutdown completo]
```

### UC-15 - Protecao de sobrecarga por mapa
```mermaid
flowchart TD
    A[Monitor tick p95/p99 por mapa] --> B{Tick p95 acima do alvo?}
    B -- nao --> C[Operacao normal]
    B -- sim --> D[Ativar modo degradado de monstros]
    D --> E{Ainda acima do alvo?}
    E -- nao --> C
    E -- sim --> F[Abrir nova instance do mapa no mesmo entry]
    F --> G[Redirecionar novos players para nova instance]
    G --> H{Ainda saturado?}
    H -- sim --> I[Aplicar fila de entrada temporaria]
    H -- nao --> C
```

### UC-16 - Pipeline QUIC de ponta a ponta
```mermaid
sequenceDiagram
    autonumber
    participant Client
    participant QuicGateway
    participant ProtocolRuntime
    participant MuCoreRuntime
    participant MapServer

    Client->>QuicGateway: frame (stream/datagram)
    QuicGateway->>ProtocolRuntime: decode frame (WireCodec)
    ProtocolRuntime-->>QuicGateway: WirePacket
    QuicGateway->>MuCoreRuntime: handle packet

    alt packet de selecao/controle
        MuCoreRuntime-->>QuicGateway: ServerMessage (MapTransfer/HelloAck/Pong)
        QuicGateway-->>Client: stream response
    else packet de gameplay
        MuCoreRuntime->>MapServer: dispatch move/skill/chat
        MapServer-->>MuCoreRuntime: side effects + persistence enqueue
    end
```

### UC-17 - Auto-scale de instance por mapa (on-demand)
```mermaid
flowchart TD
    A[SelectCharacter] --> B[WorldDirectory.select_best_map_instance]
    B --> C{Existe slot livre?}
    C -- sim --> D[Retorna route atual]
    C -- nao --> E[Acquire scale lock]
    E --> F[Revalida slot]
    F --> G{Ainda sem slot?}
    G -- nao --> D
    G -- sim --> H[Cria nova instance_id]
    H --> I[Registra rota no WorldDirectory]
    I --> J[Spawn novo MapServer]
    J --> K[Retorna route da nova instance]
```

## Contratos de consistencia
- Forte consistencia (sincrono): inventario, trade, zen/cash, rewards raras.
- Eventual consistencia (buffer + flush): posicao, hp/mp periodico, cooldown snapshot, estado de mapa.
- Idempotencia obrigatoria: todo evento critico tem `event_id` unico e reprocessavel.

## Parametros iniciais sugeridos
- `player_tick`: 50ms (20Hz)
- `monster_tick`: 100-200ms (5-10Hz)
- `flush_tick`: 2s
- `max_flush_lag`: 10-15s
- `batch_size`: 200-500 registros
- `map_ccu_soft_cap`: 250-350 players por instancia de mapa

## Evolucao sem quebra de arquitetura
1. Comecar com 1 binario (`mu-core`) no bare metal com modulos internos.
2. Extrair `MessageHub` para processo separado quando trafego de chat crescer.
3. Extrair `MonsterWorker` por mapas quentes quando AI virar gargalo.
4. Extrair `Gateway` para replicas stateless quando escalar para multi-host.
