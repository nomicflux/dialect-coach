group "default" {
  targets = ["backend-infrastructure", "backend", "frontend"]
}

target "backend-infrastructure" {
  context = "."
  dockerfile = "docker/backend-infrastructure/Dockerfile"
  tags = ["dialect-coach-backend-infrastructure:0.1.0"]
}

target "backend" {
  context = "."
  dockerfile = "backend/Dockerfile"
  contexts = {
    "dialect-coach-backend-infrastructure:0.1.0" = "target:backend-infrastructure"
  }
  tags = ["dialect-coach-backend:latest"]
  output = ["type=docker"]
}

target "frontend" {
  context = "."
  dockerfile = "frontend/Dockerfile"
  tags = ["dialect-coach-frontend:latest"]
  output = ["type=docker"]
}
