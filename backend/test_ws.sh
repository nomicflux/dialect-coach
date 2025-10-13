#!/bin/bash

# Test WebSocket with Argentinian Spanish message

MESSAGE_ID=$(uuidgen)
SESSION_ID=$(uuidgen)
TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)

MESSAGE=$(cat <<EOF
{
  "id": "$MESSAGE_ID",
  "session_id": "$SESSION_ID",
  "participant_id": "test_user",
  "content": "Hola, ¿cómo andás? Quiero aprender a hablar como un verdadero porteño.",
  "language": "es-AR",
  "timestamp": "$TIMESTAMP",
  "metadata": {
    "is_speech": false,
    "detected_dialect": null,
    "speech_confidence": null,
    "corrections": [],
    "extra": {}
  }
}
EOF
)

echo "Sending message:"
echo "$MESSAGE" | jq '.'
echo ""
echo "Connecting to WebSocket..."
echo ""

echo "$MESSAGE" | wscat -c ws://localhost:3000/ws
