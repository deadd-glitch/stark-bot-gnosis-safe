# Frontend build stage
FROM node:20-slim AS frontend-builder

WORKDIR /app/stark-frontend

# Copy frontend package files
COPY stark-frontend/package*.json ./

# Install dependencies
RUN npm ci

# Copy frontend source
COPY stark-frontend/ ./

# Build frontend
RUN npm run build

# Backend build stage
FROM rust:1.88-slim-bookworm AS backend-builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy source code
COPY . .

# Build the application
RUN cargo build --release -p stark-backend

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y ca-certificates sqlite3 && rm -rf /var/lib/apt/lists/*

# Copy the binary
COPY --from=backend-builder /app/target/release/stark-backend /app/

# Copy the built frontend (dist folder)
COPY --from=frontend-builder /app/stark-frontend/dist /app/stark-frontend/dist

# Copy the skills directory (bundled skills loaded on boot)
COPY skills /app/skills

# Expose ports (HTTP + Gateway WebSocket)
EXPOSE 8080
EXPOSE 8081

# Run the application
CMD ["/app/stark-backend"]
