#!/usr/bin/env python3

import asyncio
import websockets
import json
import uuid
from datetime import datetime, timezone

async def test_websocket():
    uri = "ws://localhost:3000/ws"

    message_id = str(uuid.uuid4())
    session_id = str(uuid.uuid4())
    timestamp = datetime.now(timezone.utc).isoformat()

    message = {
        "id": message_id,
        "session_id": session_id,
        "participant_id": "test_user",
        "content": "Hola, ¿cómo andás? Quiero aprender a hablar como un verdadero porteño.",
        "language": "es-AR",
        "timestamp": timestamp,
        "metadata": {
            "is_speech": False,
            "detected_dialect": None,
            "speech_confidence": None,
            "corrections": [],
            "extra": {}
        }
    }

    print("Connecting to", uri)
    print()

    async with websockets.connect(uri) as websocket:
        print("Connected!")
        print()
        print("Sending message:")
        print(json.dumps(message, indent=2))
        print()

        await websocket.send(json.dumps(message))

        print("Waiting for response...")
        print()

        try:
            response = await asyncio.wait_for(websocket.recv(), timeout=60.0)
            print("Received response:")
            print(json.dumps(json.loads(response), indent=2))
        except asyncio.TimeoutError:
            print("ERROR: Timeout waiting for response")
        except Exception as e:
            print(f"ERROR: {e}")

if __name__ == "__main__":
    asyncio.run(test_websocket())
