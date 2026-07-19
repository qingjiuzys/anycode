# anycode cloud account API + Account Portal SPA (+ macOS DMG in public/downloads)
# WeChat Pay PEM certs are baked into the image. TLS terminated by K8s Ingress.
# DATABASE_URL and WECHAT_PAY_API_V3_KEY must be supplied at runtime (K8s Secret / env).
#
# Before build: stage signed DMG via ./scripts/release-desktop-local.sh
# (copied by Vite public/ → dist/downloads/ → /app/portal/downloads/)

FROM node:22-bookworm AS portal
WORKDIR /portal
COPY crates/account-portal/package.json crates/account-portal/package-lock.json ./
RUN npm ci
COPY crates/account-portal/ ./
# public/downloads/*.dmg → dist/downloads/ (served at https://anycode.work/downloads/)
RUN npm run build \
    && test -d dist/downloads \
    && ls -lh dist/downloads/

FROM rust:1-bookworm AS rust
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
# account-service is not in the public workspace; inject for container build only.
RUN sed -i '/"crates\/cli",/a\    "crates/account-service",' Cargo.toml \
    && cargo build --release -p anycode-account-service

FROM debian:bookworm-slim

ARG WECHAT_PAY_APP_ID=wxd4e452a3fededd33
ARG WECHAT_PAY_MCH_ID=1703159450
ARG WECHAT_PAY_SERIAL_NO=4134FBAC10F16DF923310571D1C743EB5676D4E4
ARG WECHAT_PAY_NOTIFY_URL=https://anycode.work/api/v1/billing/webhooks/wechat
ARG ACCOUNT_PUBLIC_URL=https://anycode.work
ARG ACCOUNT_PORTAL_URL=https://anycode.work

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir -p /run/secrets/wechat \
    && chmod 755 /run/secrets/wechat

COPY deploy/account-service/secrets/apiclient_key.pem /run/secrets/wechat/apiclient_key.pem
COPY deploy/account-service/secrets/pub_key.pem /run/secrets/wechat/pub_key.pem
RUN chmod 644 /run/secrets/wechat/*.pem

COPY --from=rust /app/target/release/anycode-account /usr/local/bin/anycode-account
COPY --from=portal /portal/dist /app/portal
RUN chmod -R a+rX /app/portal

ENV ACCOUNT_SERVICE_HOST=0.0.0.0 \
    ACCOUNT_SERVICE_PORT=8080 \
    ACCOUNT_PORTAL_DIR=/app/portal \
    ACCOUNT_PUBLIC_URL=${ACCOUNT_PUBLIC_URL} \
    ACCOUNT_PORTAL_URL=${ACCOUNT_PORTAL_URL} \
    CORS_ORIGINS=https://anycode.work,http://anycode.work \
    DEFAULT_PAYMENT_PROVIDER=wechat \
    WECHAT_PAY_APP_ID=${WECHAT_PAY_APP_ID} \
    WECHAT_PAY_MCH_ID=${WECHAT_PAY_MCH_ID} \
    WECHAT_PAY_SERIAL_NO=${WECHAT_PAY_SERIAL_NO} \
    WECHAT_PAY_NOTIFY_URL=${WECHAT_PAY_NOTIFY_URL} \
    WECHAT_PAY_PRIVATE_KEY_PATH=/run/secrets/wechat/apiclient_key.pem \
    WECHAT_PAY_PUBLIC_KEY_PATH=/run/secrets/wechat/pub_key.pem \
    WECHAT_PRICE_CLOUD_5H_CENTS=9900 \
    WECHAT_PRICE_PRO_MONTHLY_CENTS=59900 \
    WECHAT_PRICE_TEAM_MONTHLY_CENTS=199900

EXPOSE 8080

USER nobody
CMD ["anycode-account"]
