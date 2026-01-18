from flask import Flask, request, jsonify

app = Flask(__name__)


@app.route("/", methods=["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"])
def echo():
    headers = {k: v for k, v in request.headers.items()}
    # Return headers as JSON so callers (the integration script) can assert values.
    return jsonify({"headers": headers}), 200


@app.route("/health")
def health():
    return "ok", 200


if __name__ == "__main__":
    app.run(host="0.0.0.0", port=19530)
