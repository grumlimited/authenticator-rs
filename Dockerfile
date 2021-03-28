FROM library/rust:1.51.0-slim-buster

RUN apt-get update && \
	apt-get install -y make bash-completion gcc \
		make \
		libsqlite3-dev \
		libgtk-3-dev \
		openssl \
		libssl-dev \
		python3 \
		python3-pip \
		python3-setuptools \
		python3-wheel \
		ninja-build \
		gettext

RUN pip3 install meson \
	mkdir -p ~/.cargo/release

WORKDIR /authenticator-rs

ENTRYPOINT ["/bin/sleep", "60000"]

