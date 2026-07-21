# anycode cloud account API + Account Portal SPA (+ macOS DMG in public/downloads)
# WeChat Pay PEM certs are baked into the image. TLS terminated by K8s Ingress.
# DATABASE_URL and WECHAT_PAY_API_V3_KEY must be supplied at runtime (K8s Secret / env).
#
# Before build: stage signed DMG via ./scripts/release-desktop-local.sh
# (copied by Vite public/ → dist/downloads/ → /app/portal/downloads/)

FROM node:22-bookworm AS portal
# Layout mirrors repo so prepare-user-docs + @anycode/site-urls resolve.
WORKDIR /src
COPY scripts/prepare-user-docs.mjs /src/scripts/prepare-user-docs.mjs
COPY docs/user /src/docs/user
COPY brand /src/brand
RUN test -f /src/brand/anycode-mark.svg && test -f /src/brand/site-urls.ts
COPY crates/account-portal/package.json crates/account-portal/package-lock.json /src/crates/account-portal/
WORKDIR /src/crates/account-portal
RUN npm ci
COPY crates/account-portal/ ./
# public/downloads/*.dmg → dist/downloads/ (served at https://anycode.work/downloads/)
RUN npm run build \
    && test -d dist/downloads \
    && ls -lh dist/downloads/

FROM rust:1-bookworm AS rust
WORKDIR /build
# account-service is a standalone crate (excluded from the public workspace).
COPY crates/account-service/Cargo.toml crates/account-service/Cargo.lock* ./
COPY crates/account-service/src ./src
COPY crates/account-service/migrations ./migrations
RUN sed -i '/^\[workspace\]/d' Cargo.toml \
    && cargo build --release

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
    && mkdir -p /app/wechat-certs \
    && chmod 755 /app/wechat-certs

# Keep PEMs under /app — NOT /run/secrets (ACK/K8s often mounts volumes there and hides image files).
COPY deploy/account-service/secrets/apiclient_key.pem /app/wechat-certs/apiclient_key.pem
COPY deploy/account-service/secrets/pub_key.pem /app/wechat-certs/pub_key.pem
RUN chmod 644 /app/wechat-certs/*.pem && chmod a+rX /app/wechat-certs

COPY --from=rust /build/target/release/anycode-account /usr/local/bin/anycode-account
COPY --from=portal /src/crates/account-portal/dist /app/portal
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
    WECHAT_PAY_PRIVATE_KEY_PATH=/app/wechat-certs/apiclient_key.pem \
    WECHAT_PAY_PUBLIC_KEY_PATH=/app/wechat-certs/pub_key.pem \
    WECHAT_PRICE_CLOUD_5H_CENTS=9900 \
    WECHAT_PRICE_PRO_MONTHLY_CENTS=59900 \
    WECHAT_PRICE_TEAM_MONTHLY_CENTS=199900

EXPOSE 8080

USER nobody
CMD ["anycode-account"]
