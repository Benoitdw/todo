FROM node:22-slim AS frontend
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY index.html vite.config.ts svelte.config.js tsconfig.json ./
COPY src ./src
RUN npm run build

FROM rust:1.86-slim AS backend
WORKDIR /app
COPY todo-server/ .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=backend /app/target/release/todo-server /usr/local/bin/
COPY --from=frontend /app/dist /static
EXPOSE 8080
CMD ["todo-server"]
