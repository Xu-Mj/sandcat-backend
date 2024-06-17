# 第一阶段：构建环境
FROM debian:buster-slim AS builder

# 替换源以加速更新和安装过程
# 设置 rdkafka 和 protobuf 版本
ARG PROTOC_VERSION=26.1
ARG LIBRDKAFKA_VERSION=2.4.0

# 安装 protoc 编译器和必要的包
RUN sed -i 's/deb.debian.org/mirrors.ustc.edu.cn/g' /etc/apt/sources.list && \
    apt-get update && \
    apt-get install -y curl gcc g++ make pkg-config libssl-dev cmake unzip && \
    apt-get clean && rm -rf /var/lib/apt/lists/*
# 安装 protoc 编译器https://github.com/protocolbuffers/protobuf/releases/download/v27.1/protoc-27.1-linux-x86_64.zip
RUN curl -OL https://github.com/protocolbuffers/protobuf/releases/download/v$PROTOC_VERSION/protoc-$PROTOC_VERSION-linux-x86_64.zip && \
    unzip -o protoc-$PROTOC_VERSION-linux-x86_64.zip -d /usr/local/bin/protoc && \
    rm protoc-$PROTOC_VERSION-linux-x86_64.zip && \
    chmod -R +x /usr/local/bin/protoc
# 安装 librdkafka
RUN curl -LO https://github.com/confluentinc/librdkafka/archive/refs/tags/v$LIBRDKAFKA_VERSION.tar.gz && \
    tar -xzf v$LIBRDKAFKA_VERSION.tar.gz && \
    cd librdkafka-$LIBRDKAFKA_VERSION && \
    ./configure && \
    make && \
    make install && \
    cd .. && \
    rm -rf librdkafka-$LIBRDKAFKA_VERSION v$LIBRDKAFKA_VERSION.tar.gz

# 安装Rust
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y

# 配置环境变量以使用Rust命令
ENV PATH="/root/.cargo/bin:${PATH}"

# 设置工作目录
WORKDIR /usr/src/sandcat-backend

## 设置 Cargo 镜像源为国内源（科大镜像）
RUN mkdir -p $HOME/.cargo && \
   echo '[source.crates-io]' > $HOME/.cargo/config.toml && \
   echo 'replace-with = "ustc"' >> $HOME/.cargo/config.toml && \
   echo '[source.ustc]' >> $HOME/.cargo/config.toml && \
   echo 'registry = "https://mirrors.ustc.edu.cn/crates.io-index"' >> $HOME/.cargo/config.toml

# 复制其余的源代码
COPY . .

# 设置环境变量
ENV PROTOC=/usr/local/bin/protoc/bin/protoc

# 构建Rust应用
RUN cargo build --release --features=static
# 第二阶段：运行环境
FROM debian:buster-slim
RUN sed -i 's/deb.debian.org/mirrors.ustc.edu.cn/g' /etc/apt/sources.list && \
    apt-get update && \
    apt-get install -y libssl1.1 && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

# 从构建阶段复制编译好的可执行文件
COPY --from=builder /usr/src/sandcat-backend/target/release/cmd /usr/local/bin/sandcat-backend


# 容器启动时运行的命令
CMD ["/usr/src/sandcat-backend/target/release/cmd", "-p", "/usr/local/bin/config.yml"]
