ARG PYTHON_VERSION=3.10
FROM python:${PYTHON_VERSION} AS build

WORKDIR /usr/src/app

# Install dependencies that require gcc
# *-slim does not ship with a C compiler
COPY ./requirements.txt .
RUN --mount=type=cache,target=/var/cache/buildkit/pip \
    pip wheel --wheel-dir /wheels -r requirements.txt

FROM python:${PYTHON_VERSION}-slim

ARG version
ENV VERSION=${version}

# Set working directory
WORKDIR /usr/src/app

# Install dependencies
COPY requirements.txt ./
COPY --from=build /wheels /wheels
RUN --mount=type=cache,target=/var/cache/buildkit/pip \
    pip install --no-cache-dir --find-links /wheels --no-index -r requirements.txt

RUN ["pip3", "install", "--no-cache-dir", "-r", "requirements.txt"]

# Copy all bot code over
COPY . .

# Run the bot when container is run
CMD ["python3", "-u", "bot.py"]
