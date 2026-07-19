from flask import Flask, jsonify, request

app = Flask(__name__)
ITEMS = []


@app.get("/items")
def list_items():
    return jsonify(ITEMS)


@app.post("/items")
def create_item():
    data = request.get_json(force=True)
    ITEMS.append(data)
    return jsonify(data), 201


if __name__ == "__main__":
    app.run(port=8765)
