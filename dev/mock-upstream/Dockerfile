FROM python:3.11-slim
WORKDIR /app
COPY app.py /app/
RUN pip install --no-cache-dir flask
EXPOSE 19530
CMD ["python", "app.py"]
