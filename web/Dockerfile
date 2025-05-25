FROM namely/protoc-all as proto-builder

COPY ./proto /proto
RUN mkdir -p /out

RUN /usr/local/bin/entrypoint.sh -d /proto -o /out -l web

FROM node:20-alpine AS builder

RUN apk add --no-cache gcompat

ARG CORE_URL

WORKDIR /workspace

COPY ./proto ./proto
COPY ./web ./web

COPY --from=proto-builder /out/ /workspace/web/src/proto/

WORKDIR /workspace/web

RUN npm install

RUN echo "CORE_URL=$CORE_URL" > .env

RUN npm run build

FROM nginx:alpine
COPY --from=builder /workspace/web/dist /usr/share/nginx/html
COPY ./web/nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
