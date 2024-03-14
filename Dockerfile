FROM rust:latest

WORKDIR /usr/src/peershare

COPY . .

RUN bash generate-certificate.sh

RUN cargo install --path .

CMD ["peershare"]