# Deploy numa VM ARM (OCI) compartilhada + Cloudflare Tunnel

Guia para rodar o monitor numa VM **OCI VM.Standard.A1.Flex** (ARM64, 3 OCPU, 18 GB, Ubuntu 24.04 aarch64) que **já roda outro app** (`<outro-app>`) com um Cloudflare Tunnel, e acessar o dashboard em `https://<app>.<seu-dominio>` com login (Cloudflare Access).

Placeholders usados: `<seu-dominio>`, `<app>` (subdomínio do monitor), `<seu-email>`, `<outro-app>` (app que já está na VM), `<tunel-existente>` (nome do túnel desse app), `<usuario>/<repo>` (repositório no GitHub).

## Como fica a VM

```
                        Cloudflare (zona <seu-dominio>)
  <outro-app>.<seu-dominio> ─┐          ┌─ <app>.<seu-dominio>  (Access: só seu e-mail)
                             ▼          ▼
               túnel "<tunel-existente>" (cloudflared via systemd no host, conexão de SAÍDA)
                             │          │ ^/api/ → 127.0.0.1:8080   resto → 127.0.0.1:3100
  ─ VM ──────────────────────┼──────────┼──────────────────────────────────────────────
   <outro-app>        127.0.0.1:3000    │   monitor (docker compose, projeto mac-mini-tracker)
   postgres do outro  127.0.0.1:5432    ├─▶ frontend 127.0.0.1:3100
                                        └─▶ backend  127.0.0.1:8080 ─▶ postgres 127.0.0.1:5433
```

- **Reaproveita o túnel e o domínio existentes.** O `cloudflared` do host passa a carregar mais um hostname, o que não exige domínio novo nem o container `cloudflared` deste repositório. O nome do túnel é só um rótulo; um túnel serve vários hostnames.
- **Portas diferentes das do outro app.** As portas do host vêm do `.env` (`POSTGRES_HOST_PORT`, `BACKEND_HOST_PORT`, `FRONTEND_HOST_PORT`), sempre em `127.0.0.1`. O PC de dev continua com 5432/8080/3000.
- **Um hostname só para o monitor.** O frontend é compilado com `NEXT_PUBLIC_API_URL=/api` e o túnel roteia `/api/*` para o backend, então tudo fica na mesma origem e não há CORS. Esse roteamento foi validado localmente com um proxy que imita a regra.
- **A API não tem autenticação própria.** O Cloudflare Access (passo 6) é o que impede qualquer pessoa de criar lojas ou apagar alvos. **Não publique sem ele.** O Access do monitor é independente do outro app.

---

## 1. Diagnóstico (confirmar o estado da VM)

```bash
echo "== SO"; . /etc/os-release; echo "$PRETTY_NAME | $(uname -m)"
echo "== recursos"; free -h | head -2; df -h / | tail -1; uptime
echo "== docker"; docker --version; docker compose version
echo "== projetos compose"; docker compose ls
echo "== portas no host"; sudo ss -tlnp | awk 'NR>1{print $4}' | sort -u
echo "== cloudflared"; systemctl is-active cloudflared; cloudflared --version
```

| Esperado | Se for diferente |
|---|---|
| `aarch64`, Docker e Compose instalados (Compose ≥ 2.24) | Instale com `curl -fsSL https://get.docker.com \| sudo sh` e rode `sudo usermod -aG docker $USER` (depois saia e entre de novo no SSH) |
| Em uso: `127.0.0.1:3000` e `127.0.0.1:5432` (outro app) | Escolha outras portas livres para o monitor no passo 3 |
| **Livres: 3100, 5433 e 8080** | Se alguma estiver ocupada, use outra e ajuste as rotas do passo 5 |
| `cloudflared` = `active` | Sem ele, veja o [Apêndice A](#apêndice-a--vm-sem-cloudflared-túnel-em-container) |
| Disco livre ≥ 15 GB | Libere espaço: `docker system prune` remove imagens e caches **sem uso**; volumes ficam intactos |

## 2. Código no GitHub e na VM

### 2.1 No seu PC: enviar o repositório

```powershell
cd <pasta-do-repo>
git remote add origin git@github.com:<usuario>/<repo>.git   # ou https://github.com/...
git push -u origin main
```

### 2.2 Na VM: deploy key própria

O GitHub **não aceita a mesma deploy key em dois repositórios**, e a `~/.ssh/id_ed25519` pode já ser de outro repo. Crie outra chave e um alias SSH:

```bash
ssh-keygen -t ed25519 -C "vm-mac-mini-tracker" -f ~/.ssh/id_mac_tracker -N ""
cat >> ~/.ssh/config <<'EOF'

Host github-mac-tracker
    HostName github.com
    User git
    IdentityFile ~/.ssh/id_mac_tracker
    IdentitiesOnly yes
EOF
chmod 600 ~/.ssh/config
cat ~/.ssh/id_mac_tracker.pub
```

Cole a chave em GitHub → `<repo>` → **Settings → Deploy keys → Add deploy key**, **sem** *Allow write access*. Depois:

```bash
ssh -T github-mac-tracker        # "Hi <usuario>/<repo>! You've successfully authenticated..."
git clone github-mac-tracker:<usuario>/<repo>.git ~/mac-mini-tracker
cd ~/mac-mini-tracker
```

A pasta `~/mac-mini-tracker` define o nome do projeto compose (`mac-mini-tracker`). Assim, rede e volume (`mac-mini-tracker_default`, `mac-mini-tracker_pgdata`) não se misturam com os de outros projetos.

## 3. `.env` da VM

```bash
cd ~/mac-mini-tracker
PASS=$(openssl rand -hex 24)        # hex: @ / : quebrariam a DATABASE_URL
cat > .env <<EOF
POSTGRES_USER=mactracker
POSTGRES_PASSWORD=$PASS
POSTGRES_DB=mactracker_db
DATABASE_URL=postgres://mactracker:$PASS@localhost:5433/mactracker_db
RUST_LOG=info
NEXT_PUBLIC_API_URL=http://localhost:8080/api
PUBLIC_API_URL=/api
POSTGRES_HOST_PORT=5433
BACKEND_HOST_PORT=8080
FRONTEND_HOST_PORT=3100
TUNNEL_TOKEN=
EOF
chmod 600 .env
grep -c "$PASS" .env && unset PASS   # deve imprimir 2
```

- O compose monta a `DATABASE_URL` do backend a partir de `POSTGRES_*`, com o host `postgres` da rede interna. A `DATABASE_URL` do `.env` só serve para rodar o cargo fora do Docker.
- `PUBLIC_API_URL=/api` é lido **no build** do frontend. Se mudar, rode `docker compose build frontend && docker compose up -d frontend`.
- `TUNNEL_TOKEN` fica vazio: nesta VM quem faz o túnel é o `cloudflared` do host.
- Mercado Livre: copie `ML_CLIENT_ID`, `ML_CLIENT_SECRET` e o `ML_REFRESH_TOKEN` **atual** do PC. O backend só lê o refresh token do `.env` na primeira vez; depois ele vive na tabela `oauth_tokens` e muda a cada renovação. Ele é de uso único: só um banco pode usá-lo, então depois de copiar não rode o scraper do ML no PC (ou gere outro token para o dev).

## 4. Primeira subida

O build compila Rust + BoringSSL e Next.js em ARM64 e leva **~12–15 min**, ocupando as 3 OCPU. Durante esse tempo, o outro app pode ficar lento: **faça num horário de pouco uso.**

```bash
cd ~/mac-mini-tracker
docker compose up --build -d          # sem --profile tunnel: o container cloudflared NÃO sobe aqui
docker compose ps                     # postgres healthy; backend e frontend Up
curl -s http://127.0.0.1:8080/api/health            # {"status":"ok"}
curl -sI http://127.0.0.1:3100 | head -1            # HTTP/1.1 200 OK
sudo ss -tlnp | grep -E ':(3100|5433|8080)\b'       # tudo em 127.0.0.1, nada em 0.0.0.0
docker compose logs -f backend        # "migrations ok" e, ~10 s depois, "varredura concluída" (Ctrl+C sai)
```

Confira também que o outro app não foi afetado: `curl -sI http://127.0.0.1:3000 | head -1` deve continuar `200`.

## 5. Rotas no túnel existente (painel da Cloudflare)

**Rede → Túneis → `<tunel-existente>` → aba Rotas → Adicionar rota → Aplicativo publicado.** Crie duas rotas:

| Ordem final | Subdomínio | Domínio | Caminho | URL do serviço |
|---|---|---|---|---|
| A | `<app>` | `<seu-dominio>` | `^/api/` | `http://127.0.0.1:8080` |
| B | `<app>` | `<seu-dominio>` | *(vazio)* | `http://127.0.0.1:3100` |

- Use **`127.0.0.1`** e não `localhost`: `localhost` pode resolver para IPv6.
- Na segunda rota vai aparecer *"An A, AAAA, or CNAME record with that host already exists"*. É **esperado**: o CNAME `<app>` foi criado pela primeira rota.
- ⚠️ **A ordem importa.** As rotas são avaliadas de cima para baixo, e cada nova entra por último. Na aba **Rotas**, arraste a rota `<app>…/^/api/` (alça ⋮⋮) para **acima** da rota `<app>.<seu-dominio>` sem caminho. As rotas do outro app podem ficar onde estão, porque são de outro hostname.

Teste do PC (antes do Access, deve responder direto):
```bash
curl -s https://<app>.<seu-dominio>/api/health          # {"status":"ok"}
curl -s -o /dev/null -w "%{http_code}\n" https://<app>.<seu-dominio>/   # 200
```

## 6. Cloudflare Access para `<app>.<seu-dominio>` (obrigatório, logo em seguida)

1. Zero Trust → **Access → Policies → Add a policy**:
   - **Name:** `owner-only`
   - **Action:** Allow
   - **Session duration:** 1 semana
   - **Include → Emails:** `<seu-email>`
2. **Access → Applications → Add an application → Self-hosted**:
   - **Name:** `Mac mini Tracker`
   - **Public hostname:** `<app>` + `<seu-dominio>`, **path vazio**, para cobrir o dashboard e a `/api`
   - **Policy:** `owner-only`
   - **Login method:** One-time PIN
3. Teste numa **aba anônima**: `https://<app>.<seu-dominio>` deve pedir o e-mail e mandar um código. Pelo terminal:
   ```bash
   curl -sI https://<app>.<seu-dominio>/api/health | head -3   # 302 → <team>.cloudflareaccess.com
   ```

Se os nomes dos menus estiverem diferentes no painel em português, use a busca do painel (Ctrl+K) com "Access" / "Aplicativos".

## 7. Operação

| Tarefa | Comando (em `~/mac-mini-tracker`) |
|---|---|
| Atualizar após `git push` | `git pull --ff-only && docker compose up --build -d` |
| Ver erros de scraping | `docker compose logs --since 1h backend \| grep -E "WARN\|varredura"` |
| Status | `docker compose ps` |
| Parar / subir | `docker compose stop` / `docker compose up -d` (os dados ficam no volume) |
| Varredura manual | Botão **Atualizar Preços Agora** no dashboard |

As atualizações rebuildam só o que mudou (camadas em cache). Mudar só o frontend leva ~2 min; mudar dependências do Rust, ~10 min.

**Backup diário:** rode `mkdir -p ~/backups/mac-mini-tracker` e depois `crontab -e`:
```cron
0 4 * * * cd $HOME/mac-mini-tracker && docker compose exec -T postgres pg_dump -U mactracker mactracker_db | gzip > $HOME/backups/mac-mini-tracker/$(date +\%F).sql.gz && find $HOME/backups/mac-mini-tracker -name '*.sql.gz' -mtime +14 -delete
```

**Restaurar** num banco vazio:
```bash
gunzip -c ARQUIVO.sql.gz | docker compose exec -T postgres psql -U mactracker -d mactracker_db
```

**Reinício da VM:** o Docker sobe no boot, os serviços têm `restart: unless-stopped` e o `cloudflared` é systemd. Tudo volta sozinho.

## 8. Riscos e problemas conhecidos

- **IP de datacenter:** Amazon e KaBuM desconfiam mais de IP de nuvem do que de IP residencial. Os 0 erros obtidos no PC podem não se repetir. Acompanhe os `WARN` nos primeiros dias. Se aparecerem 503 constantes, aumente o `scrape_interval_minutes` da loja na tela **Lojas** e considere atualizar o `wreq-util` para um perfil de Chrome mais novo (`Emulation::Chrome…` em `backend/src/scrapers/mod.rs`).
- **CPU compartilhada:** fora dos builds, o monitor usa pouco (uma varredura a cada 30 min por loja, < 1 GB de RAM). Os builds são o único momento de disputa com o outro app.
- **Recuperação de instância Always Free:** a Oracle pode recuperar VMs Always Free ociosas. Com o scheduler a cada 5 min, o risco é baixo. *Pay As You Go* elimina de vez.
- **Testes do backend** no PC precisam da imagem com cmake/clang: `docker build --target chef -t mmt-rust ./backend`. O teste de banco (`persist_guarda_extremos_e_historico_sem_duplicar`) é `#[ignore]`: suba o Postgres (`docker compose up -d postgres`), exporte `DATABASE_URL` e rode `cargo test -- --ignored`.
- **Build no Windows via Git Bash:** passar `--build-arg NEXT_PUBLIC_API_URL=/api` direto no `docker build` sofre conversão de caminho do MSYS (vira `C:/Program Files/Git/api`). Use `MSYS_NO_PATHCONV=1` ou o compose, que lê do `.env`. Na VM Linux isso não ocorre.

| Sintoma | Causa provável |
|---|---|
| `port is already allocated` no `up` | Porta do `.env` em uso (talvez pelo outro app) → escolha outra e ajuste a rota do passo 5 |
| Dashboard abre mas mostra "Failed to fetch" | Frontend compilado sem `PUBLIC_API_URL=/api` → `docker compose build frontend && docker compose up -d frontend` |
| `/api/...` devolve a página do Next (HTML) ou 404 | Ordem das rotas: `<app>…/^/api/` precisa estar acima de `<app>.<seu-dominio>` |
| **502** em `<app>.<seu-dominio>` | Container parado ou porta errada na rota → `docker compose ps`; `curl -I http://127.0.0.1:3100` na VM |
| Erro **1033** | `cloudflared` parado → `sudo systemctl restart cloudflared` (afeta todos os hostnames do túnel por alguns segundos) |
| Backend reiniciando em loop | `docker compose logs backend`; em geral, senha do `.env` diferente da usada quando o volume foi criado (ela só vale na primeira criação) |

---

## Apêndice A — VM sem `cloudflared` (túnel em container)

Para uma VM sem túnel no host, este repositório traz um serviço `cloudflared` no compose (profile `tunnel`), que fala com os containers pelo nome:

1. Zero Trust → **Rede → Túneis → Criar túnel (Cloudflared)** → instalação **Docker** → copie só o token (`eyJ…`).
2. No `.env`: `TUNNEL_TOKEN=eyJ...`, e as portas podem ficar no padrão.
3. `docker compose --profile tunnel up --build -d`. Os logs devem mostrar "Registered tunnel connection".
4. Rotas: `^/api/` → `http://backend:8080` e, abaixo dela, sem caminho → `http://frontend:3000`.
5. Access igual ao passo 6.
6. Para atualizar: `git pull && docker compose --profile tunnel up --build -d`.
