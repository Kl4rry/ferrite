FROM quay.io/pypa/manylinux2014_x86_64 AS build

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rustup_init.sh
RUN sh rustup_init.sh -y

ENV PATH="/root/.cargo/bin:$PATH"

RUN USER=root cargo new --bin ferrite
WORKDIR /ferrite

COPY . .

RUN cargo fetch
RUN cargo build --release --all-features

FROM ubuntu:latest

COPY --from=build /ferrite/target/release/ferrite ./ferrite

CMD ["echo"]