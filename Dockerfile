FROM python:3.13.9-alpine

ENV TZ=Asia/Shanghai
VOLUME ["/config", "/logs", "/media","/fonts"]

RUN apk update
RUN apk add --no-cache build-base linux-headers tzdata

COPY requirements.txt requirements.txt
RUN pip install -r requirements.txt && \
    rm requirements.txt

COPY app /app

RUN rm -rf /tmp/*

ENTRYPOINT ["python", "/app/main.py"]