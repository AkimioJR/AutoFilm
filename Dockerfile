FROM python:3.12.7-alpine AS builder
WORKDIR /builder

RUN apk update && \
    apk add --no-cache \
    build-base \
    linux-headers

# 安装构建依赖
RUN pip install --no-cache-dir --upgrade pip && \
    pip install --no-cache-dir cython setuptools

COPY setup.py setup.py
COPY app ./app

RUN python setup.py

RUN apk del build-base linux-headers && \
    find app -type f \( -name "*.py" ! -name "main.py" ! -name "__init__.py" -o -name "*.c" \) -delete

FROM python:3.12.7-alpine

ENV TZ=Asia/Shanghai
VOLUME ["/config", "/logs", "/media", "/fonts"]
EXPOSE 8000

# 添加运行时依赖
RUN apk update && \
    apk add --no-cache \
    tzdata \
    curl \
    build-base \
    linux-headers \
    libgomp && \
    cp /usr/share/zoneinfo/${TZ} /etc/localtime && \
    echo ${TZ} > /etc/timezone

# 安装Python依赖
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt \
    -i https://pypi.tuna.tsinghua.edu.cn/simple \
    --default-timeout=100 \
    && rm requirements.txt \
    && pip install fastapi==0.109.0 uvicorn==0.27.0 --force-reinstall

# 清理构建工具
RUN apk del build-base linux-headers && \
    rm -rf /var/cache/apk/*

# 从构建阶段复制应用
COPY --from=builder /builder/app /app

# 最终清理
RUN apk del curl && \
    rm -rf /var/cache/apk/* && \
    rm -rf /tmp/*

ENTRYPOINT ["python", "/app/main.py", "--host", "0.0.0.0", "--port", "8000"]