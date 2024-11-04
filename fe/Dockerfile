FROM node:22-bullseye AS base
WORKDIR /code
COPY . /code
RUN npm install
RUN npm run build

FROM nginx:1.27.2 AS production
EXPOSE 80
COPY --from=base /code/dist /usr/share/nginx/html
