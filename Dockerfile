# Using the latest ubuntu (Same as production enviroment)
FROM ubuntu:latest

# Workdir 
WORKDIR /aurr

# Copy of essential files. This will mainly be the data/ dire and the binary itself
COPY aurr-manager/data /aurr/.

 
COPY aurr-manager/target/debug/aurr-manager /aurr/aurr
COPY aurr-manager/Config.toml /aurr/Config.toml

RUN chmod +x /aurr/aurr




