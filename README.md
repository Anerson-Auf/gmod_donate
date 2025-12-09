# GMod TCP

Система обмена сообщениями между сервером и клиентами через TCP/HTTP протоколы.

## Компоненты

- **server** - TCP сервер для клиентов + HTTP API сервер для веб-приложения
- **client** - Модуль для Garry's Mod (можно адаптировать под другие платформы)
- **client_app** - Десктопное приложение для управления донатами
- **shared** - Общие типы данных и протоколы

## Настройка сервера

Создайте файл `server/.env`:

```env
# TCP сервер (для клиентов)
HOST=0.0.0.0
PORT=25565

# HTTP API сервер
API_HOST=0.0.0.0
API_PORT=8080

# CORS
ALLOWED_ORIGINS=*

# API пароль (обязательно для защиты), можете через запятую указывать ряд паролей.
# Приложением пользователься полноценно без паролей не выйдет.
API_PASSWORDS=your_password_here

# Не забудьте указать в client_app/.env (создайте этот файл) API_URL
# Пример: API_URL = http://localhost:8080
# Если сертификаты не указаны, сервер автоматически создаст самоподписанные
# В client_app/.env укажите API_URL=https://your-domain.com:443 для HTTPS
```

### Запуск

```bash
cd server
cargo run --release
```

## Клиент для Garry's Mod

Клиент (`client/`) реализован как модуль для Garry's Mod, но может быть адаптирован под любые другие цели.
Основные функции позволяют это сделать при желании, единственная проблема адаптивности в данный момент - поиск uuid и tcp в .txt файлах, которые лежат в директориях гмода.
Основная логика работы:

- Регистрация клиента на сервере
- Периодический опрос сервера на новые сообщения
- Передача сообщений в Lua через глобальные функции

### Lua API

- `GModTCPGetMessages()` - получить сообщения из очереди (возвращает таблицу или nil)
- `GModTCPPollNow()` - принудительно запросить сообщения с сервера

## HTTPS
Работает с помощью nginx.

## **Быстрый** старт на linux

Server надо деплойнуть на машине.

```bash
sudo apt update -y && sudo apt upgrade -y
sudo apt install build-essential
curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh # 1
git clone https://github.com/Anerson-Auf/gmod_donate
cd gmod_donate
# Можете воспользоваться deploy_server.sh от этой точки, выполнит все дальнейшие действия для деплоя сервера.
# От этой строчки идёт принцип сборки модулей
cargo build --release -p gmod_tcp_server
mkdir ../server && mv target/release/gmod_tcp_server ../server/ && cd ../server
nano .env
```
Внутрь .env
```bash
HOST=0.0.0.0
PORT=25565
API_HOST=0.0.0.0
API_PORT=9060
ALLOWED_ORIGINS=*
API_PASSWORDS=test
```
Далее можно запускать, сервер работает на http://127.0.0.1:9060

`./gmod_tcp_server`

### gmod_tcp_app
По схожему принципу собрать 
.env рядом
```bash
API_URL=http://127.0.0.1:9060
API_PASSWORD=test
```

### gmod_tcp_client
В нынешнем формате для Garry's Mod требуется создать host.txt, uuid.txt.
Разместить их в ./common/GarrysMod/data/gmod_tcp

host:
`PUBLIC_IP:25565`
uuid:
`local-test # Любой на ваш выбор`


## Сборка

```bash
# Весь проект
cargo build --release

# Только сервер
cd server && cargo build --release # или cargo build --release -p  gmod_tcp_server

# Только клиент
cd client && cargo build --release # или cargo build --release -p gmod_tcp_client

# Только приложение
cd client_app && cargo build --release # или cargo build --release -p gmod_tcp_app
```

