FROM node:22-bullseye AS base
WORKDIR /code
COPY . /code
ARG VITE_API_HOST
RUN npm install
RUN npm run build

FROM nginx:1.27.2 AS production
EXPOSE 80
COPY --from=base /code/dist /usr/share/nginx/html
