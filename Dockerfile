FROM quay.io/pypa/manylinux2014_x86_64

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rustup_init.sh
RUN sh rustup_init.sh -y

COPY USER=root cargo new --bin ferrite
WORKDIR ferrite

COPY . .

RUN cargo build --release