# Blastfile

*A lightweight, self-hosted file transfer service built in Rust. Upload files from the CLI, get a temporary shareable URL, and download them anywhere with `curl` or `wget`.*

Blastfile is a micro HTTP server written in Rust (Axum) that allows you to **upload a file via PUT** and **serve it for reading** on a public URL. It's simple, fast, and **self-hostable**. Perfect for sharing a one-off file without the hassle of a large stack.

---

## Features

* **Direct upload** from the terminal: `curl -T file.pdf https://…`
* Immediate **download URL** (`/files/<name>`), copyable and scriptable
* **Configurable max size** (`MAX_BYTES`, 1 GiB by default)
* Optional **upload password** via `x-upload-password` header
* **Static server** for uploaded files via `/files/…`
* **Health endpoint** `/health`

---

## API & conventions

### Endpoints

* `PUT /{filename}` — file upload
* `GET /files/{filename}` — download
* `GET /health` — return `ok`

### File name constraints

* Allowed regex: `^[A-Za-z0-9._-]{1,200}$`
* Prohibited: `..`, `/`, `\`

### Headers

* `x-upload-password: <secret>` — Required **only** if `UPLOAD_PASSWORD` env variable is defined.

### Response body (success)

```
Upload OK
wget: wget 'https://example.org/files/awesome-file.pdf'
size: 123456 bytes
```

---

## Usage (CLI)

> **HTTPS & redirection tip:** if your domain forces HTTPS behind a reverse proxy, use `https://…` or `curl -L` **directly** to follow the redirection **with a final `/`**.

### Uploader

Explicit name in the URL (recommended):

```bash
curl -T awesome-file.pdf https://filer.example.org/awesome-file.pdf \
-H 'x-upload-password: YOUR_SECRET'   # if activated
```

Let `curl` add the local name (note the trailing `/` and `-L`):

```bash
curl -L -T awesome-file.pdf https://filer.example.org/ \
  -H 'x-upload-password: YOUR_SECRET'
```

### Download

```bash
# with curl
curl -LO https://filer.example.org/files/awesome-file.pdf

# with wget
wget https://filer.example.org/files/awesome-file.pdf
```

### Check health

```bash
curl -s https://filer.example.org/health
# ok
```

---

## Configuration (environment variables)

| Variable          |              Default | Description                                                                                                                                                                                     |
| ----------------- |---------------------:| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `BIND`            |       `0.0.0.0:8080` | Server listening address                                                                                                                                                                     |
| `DATA_DIR`        |              `/data` | File storage folder                                                                                                                                                                |
| `PUBLIC_BASE_URL` |            *(empty)* | Public base used to generate URLs in responses. If empty → fallback `http://localhost:<port>`. If the URL does not have a scheme, **`https://` is added**. The trailing `/` is removed. |
| `MAX_BYTES`       | `1073741824` (1 GiB) | Maximum size accepted per upload                                                                                                                                                             |
| `UPLOAD_PASSWORD` |            *(empty)* | Optional secret. If set, the `x-upload-password` header is **required**.                                                                                                            |

**Examples:**

* `PUBLIC_BASE_URL=filer.example.org` → normalized to `https://filer.example.org`
* `PUBLIC_BASE_URL=http://192.0.2.10:8080/` → `http://192.0.2.10:8080`

---

## Deployment with Docker — `docker run`

```bash
# Build the image from this repository
docker build -t blastfile:latest .

# Create a persistent folder on the host side
sudo mkdir -p /opt/blastfile/data
sudo chown 10001:10001 /opt/blastfile/data  # if non-root image with UID 10001

# Start
docker run -d \
  --name blastfile \
  -p 8080:8080 \
  -e PUBLIC_BASE_URL="https://filer.example.org" \
  -e UPLOAD_PASSWORD="change-me" \
  -e MAX_BYTES=$((2*1024*1024*1024)) \
  -v /opt/blastfile/data:/data \
  --restart unless-stopped \
  blastfile:latest
```

> **Permissions**: If the runtime image uses a **non-root user** (recommended), ensure that the mounted volume is **writable** by that UID (e.g., `10001`).

Start:

```bash
docker compose up -d --build
```

---

## Deploy on Coolify

> Example for deployment from a Git repository containing this project **and** a multi-stage Dockerfile.

1. **Add an application** → *Add New* → *Application* → *Git Repository* (main branch).
2. **Build Pack**: choose **Dockerfile** (path `./Dockerfile`, context `.`).
3. **Internal port**: 8080 (corresponds to `BIND=0.0.0.0:8080`).
4. **Domain**: attach `https://filer.example.org` to the application (Coolify manages the proxy/SSL).
5. **Env. variables** (*Settings → Environment Variables*):

* `PUBLIC_BASE_URL=https://filer.example.org`
* `DATA_DIR=/data`
* `BIND=0.0.0.0:8080`
* *(optional)* `UPLOAD_PASSWORD=<long_secret_random>`
* *(optional)* `MAX_BYTES=2147483648` (e.g., 2 GiB)
6. **Volumes** (*Persistent Storage*): add a named volume and **mount it on `/data`**.
7. **Healthcheck** : Disabled
8. **Deploy**.

### Test commands after deployment

```bash
# Upload (HTTPS)
curl -L -T awesome-file.pdf https://filer.example.org/ # add password header if needed

# Download
wget https://filer.example.org/files/awesome-file.pdf
```

---

## Safety & best practices

* **Always use HTTPS** in production (managed by Coolify/Traefik/Caddy/Nginx on the front end).
* Enable `UPLOAD_PASSWORD` with a **long, random value** if the service is not strictly private.
* Monitor and purge `/data` regularly if necessary (Blastfile does **not** set automatic expiration yet).
* Restrict file names on the client side if you auto-generate names.

---

## ️ Local development

```bash
# Start locally
BIND=127.0.0.1:8080 DATA_DIR=./data cargo run

# Local upload
curl -T awesome-file.pdf http://127.0.0.1:8080/
```

---

## License

This project is distributed under the MIT license.
