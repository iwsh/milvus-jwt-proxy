from fastapi import FastAPI, Request

app = FastAPI()


@app.api_route("/", methods=["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"])
async def echo(request: Request):
    headers = {k: v for k, v in request.headers.items()}
    return {"headers": headers}


@app.get("/health")
async def health():
    return "ok"


if __name__ == "__main__":
    # Run with hypercorn for h2/h2c support when executed directly
    import asyncio
    from hypercorn.asyncio import serve
    from hypercorn.config import Config

    config = Config()
    config.bind = ["0.0.0.0:19530"]
    config.h2 = True

    asyncio.run(serve(app, config))
