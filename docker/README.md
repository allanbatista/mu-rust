# Mu Online - Docker Infrastructure

Esta pasta contém a infraestrutura Docker necessária para executar o projeto Mu Online, incluindo MongoDB e ferramentas de gerenciamento.

## Serviços Disponíveis

### 1. MongoDB
- **Imagem**: `mongo:7.0`
- **Porta**: `27017`
- **Container**: `mu-mongodb`
- Banco de dados principal para armazenar dados do jogo

### 2. Mongo Express (Web UI)
- **Imagem**: `mongo-express:latest`
- **Porta**: `8081`
- **Container**: `mu-mongo-express`
- Interface web para gerenciar o MongoDB

## Como Usar

### 1. Configuração Inicial

Copie o arquivo de exemplo `.env.example` para `.env` e ajuste as credenciais:

```bash
cp .env.example .env
```

Edite o arquivo `.env` conforme necessário:

```env
# MongoDB Configuration
MONGO_ROOT_USERNAME=admin
MONGO_ROOT_PASSWORD=sua_senha_segura
MONGO_DATABASE=mu

# Mongo Express Configuration (Web UI)
MONGO_EXPRESS_USERNAME=admin
MONGO_EXPRESS_PASSWORD=sua_senha_ui
```

### 2. Iniciar os Serviços

```bash
# Iniciar todos os serviços em background
docker-compose up -d

# Ver logs dos serviços
docker-compose logs -f

# Ver logs de um serviço específico
docker-compose logs -f mongodb
```

### 3. Parar os Serviços

```bash
# Parar os serviços (mantém volumes)
docker-compose stop

# Parar e remover containers (mantém volumes)
docker-compose down

# Parar, remover containers E volumes (CUIDADO: apaga dados!)
docker-compose down -v
```

### 4. Acessar os Serviços

#### MongoDB
Conexão direta via cliente MongoDB:
```bash
mongosh "mongodb://admin:admin123@localhost:27017/mu?authSource=admin"
```

String de conexão para aplicação:
```
mongodb://admin:admin123@localhost:27017/mu?authSource=admin
```

#### Mongo Express (Web UI)
Abra no navegador:
```
http://localhost:8081
```

Credenciais padrão (definidas no `.env`):
- Username: `admin`
- Password: `pass`

## Estrutura do Banco de Dados

O script `mongo-init.js` cria automaticamente as seguintes coleções com índices otimizados:

### Coleções

1. **users** - Dados de usuários do jogo
   - Índices: `username` (único), `email` (único), `created_at`

2. **characters** - Personagens dos jogadores
   - Índices: `user_id`, `name` (único), `level`, `class`

3. **guilds** - Guildas do jogo
   - Índices: `name` (único), `master_id`

4. **items** - Itens dos personagens
   - Índices: `character_id`, `item_type`

5. **game_sessions** - Sessões ativas de jogo
   - Índices: `user_id`, `session_token` (único)
   - TTL: 24 horas (sessões expiram automaticamente)

## Healthcheck

O MongoDB inclui verificação de saúde automática:
- Intervalo: 10 segundos
- Timeout: 5 segundos
- Tentativas: 5
- Período inicial: 40 segundos

O Mongo Express só inicia após o MongoDB estar saudável.

## Volumes Persistentes

Os dados são armazenados em volumes Docker nomeados:
- `mongodb_data` - Dados do banco
- `mongodb_config` - Configurações do MongoDB

Para fazer backup dos dados:
```bash
docker run --rm -v mongodb_data:/data -v $(pwd):/backup alpine tar czf /backup/mongodb-backup.tar.gz /data
```

Para restaurar:
```bash
docker run --rm -v mongodb_data:/data -v $(pwd):/backup alpine tar xzf /backup/mongodb-backup.tar.gz -C /
```

## Troubleshooting

### MongoDB não inicia
```bash
# Verificar logs
docker-compose logs mongodb

# Verificar se a porta está em uso
sudo lsof -i :27017

# Recriar containers
docker-compose down
docker-compose up -d
```

### Resetar banco de dados
```bash
# CUIDADO: Apaga todos os dados!
docker-compose down -v
docker-compose up -d
```

### Verificar status dos serviços
```bash
docker-compose ps
```

## Segurança

⚠️ **IMPORTANTE**:
- Nunca use credenciais padrão em produção
- Não commite o arquivo `.env` no Git
- Use senhas fortes para usuários admin
- Em produção, restrinja acesso às portas via firewall

## Network

Todos os serviços estão conectados à rede `mu-network` (bridge), permitindo comunicação interna entre containers usando nomes de serviço.

## Requisitos

- Docker Engine 20.10+
- Docker Compose 2.0+
- Mínimo 2GB RAM disponível
- Mínimo 5GB espaço em disco

## Comandos Úteis

```bash
# Ver uso de recursos
docker stats

# Limpar containers parados
docker-compose rm

# Reconstruir imagens
docker-compose build

# Executar comando no MongoDB
docker-compose exec mongodb mongosh

# Ver configuração final do compose
docker-compose config
```
