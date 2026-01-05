# Deployment Platform Selection - Dialect Coach

**Status**: Research Complete - Awaiting Decision

**Date Created**: 2025-12-16

**Research Sources**: Online research conducted December 2025 for current pricing/features

---

## Current State

**Local Docker Compose Stack**:
- Frontend (nginx + WASM)
- Backend (Rust/Axum)
- Postgres (Docker container)

**Problem**: Postgres is local only. No multi-instance scaling capability. Need managed Postgres + container hosting for production.

---

## Platform Comparison

### 1. Render

**Strengths**:
- Best docker-compose compatibility via "Blueprints" (YAML-based IaC)
- Can translate our existing docker-compose.yml to production easily
- Native Rust support + Docker support
- Easiest developer experience
- Git-based deployments (auto-deploy on push)
- Managed Postgres with backups/HA

**Pricing** (2025):
- Web Services: $0/month (free tier) to $19/month (Professional)
- Managed Postgres: $0/month (starter, with limitations) to paid plans at $0.30/GB
- 100 GB/month bandwidth included
- $0.25/GB for SSD storage

**Estimated Monthly Cost**:
- Free tier: $0 (but services spin down after inactivity - dealbreaker for production)
- Paid tier: ~$19/month (Professional) + Postgres storage costs
- Small production setup: ~$25-35/month

**Weaknesses**:
- Free tier spins down services with inactivity
- Less control than Fly.io for advanced networking

**Best For**: Teams wanting simple, managed infrastructure with docker-compose-like deployments

**Sources**:
- [Render Pricing](https://render.com/pricing)
- [Render Postgres Docs](https://render.com/docs/postgresql)
- [Railway vs Render Comparison](https://northflank.com/blog/railway-vs-render)

---

### 2. Railway

**Strengths**:
- Fast deployment from GitHub
- Simple git-centric workflow
- Usage-based pricing (pay only for what you use)
- Good for prototypes and small apps
- Managed Postgres included (no separate pricing)
- Nixpacks auto-detects and builds containers

**Pricing** (2025):
- $5/month free credits (not a true free tier)
- Usage-based: RAM hours, CPU hours, storage
- Billed per minute (cost-effective for variable workloads)

**Estimated Monthly Cost**:
- Small app + Postgres: ~$12/month
- Variable based on actual usage

**Weaknesses**:
- No true free tier (ended August 2023)
- Less suitable for high-traffic production apps
- Limited geographic distribution

**Best For**: Rapid prototyping, internal tools, hobby projects with moderate traffic

**Sources**:
- [Railway Pricing](https://railway.com/pricing)
- [Railway PostgreSQL Docs](https://docs.railway.com/guides/postgresql)
- [Railway vs Render Comparison](https://northflank.com/blog/railway-vs-render)

---

### 3. Fly.io

**Strengths**:
- Edge deployment (global distribution, low latency)
- Full Docker container support
- Static IPs available
- Best for globally distributed apps
- Advanced networking capabilities

**Pricing** (2025):
- NO free tier (removed in 2024)
- Managed Postgres: ~$38/month minimum (entry pricing)
- Storage: $0.15/GB per month
- Bandwidth: $0.02/GB
- Standard support: $29/month

**Estimated Monthly Cost**:
- Minimum viable setup: ~$50-80/month
- Postgres alone is expensive compared to competitors

**Weaknesses**:
- Steeper learning curve
- CLI-heavy workflow
- Significantly higher cost for Postgres
- No free tier for experimentation

**Best For**: Apps requiring global edge deployment, low latency across regions, advanced networking

**Sources**:
- [Fly.io Pricing](https://fly.io/docs/about/pricing/)
- [Managed Postgres Pricing Discussion](https://community.fly.io/t/managed-postgres-pricing/25734)
- [Platform Comparison](https://medium.com/ai-disruption/railway-vs-fly-io-vs-render-which-cloud-gives-you-the-best-roi-2e3305399e5b)

---

## Decision Matrix

| Feature | Render | Railway | Fly.io |
|---------|--------|---------|--------|
| **Docker Support** | ✅ Excellent (Blueprints) | ✅ Good (Nixpacks) | ✅ Excellent (Native) |
| **Rust Support** | ✅ Native | ⚠️ Via Docker | ✅ Via Docker |
| **Managed Postgres** | ✅ $0-0.30/GB | ✅ Included in usage | ⚠️ $38/month minimum |
| **Free Tier** | ⚠️ Yes (spins down) | ⚠️ $5 credits | ❌ No |
| **Est. Monthly Cost** | $25-35 | $12-20 | $50-80 |
| **Learning Curve** | ⭐ Easy | ⭐⭐ Moderate | ⭐⭐⭐ Steep |
| **docker-compose Migration** | ⭐⭐⭐ Excellent | ⭐⭐ Good | ⭐ Manual |
| **Geographic Distribution** | ⭐⭐ Regional | ⭐⭐ Regional | ⭐⭐⭐ Global Edge |
| **Git-based Deploy** | ✅ Yes | ✅ Yes | ⚠️ CLI-focused |

---

## Recommendations

### For Dialect Coach Specifically:

**Recommended: Railway** (Best balance for current stage)

**Reasoning**:
1. **Cost-effective**: ~$12/month for small production setup (backend + frontend + Postgres)
2. **Usage-based pricing**: Pay only for actual usage (good for variable traffic)
3. **Simple deployment**: Git-based, minimal configuration needed
4. **Our docker-compose.yml can be used as reference**: Railway can deploy from Dockerfile
5. **Low commitment**: Month-to-month, easy to migrate later if needs change

**Alternative: Render** (If you prefer managed simplicity)

**Reasoning**:
1. **Easiest migration**: Blueprints translate docker-compose.yml directly
2. **Managed services**: Less operational overhead
3. **Slightly higher cost**: ~$25-35/month but very predictable
4. **Good for production**: Better HA and managed database features

**Not Recommended: Fly.io** (Too expensive for current stage)

**Reasoning**:
1. **High cost**: $50-80/month minimum (4-6x Railway's cost)
2. **Postgres pricing**: $38/month alone is more than entire Railway stack
3. **Overkill**: Global edge distribution not needed for language learning app
4. **Learning curve**: Significant time investment for CLI-heavy workflow

---

## Migration Path (Railway Example)

### Phase 1: Database Migration
1. Create Railway Postgres instance
2. Export local Postgres data (if any test data exists)
3. Update DATABASE_URL environment variable
4. Verify schema initialization (init.sql runs automatically)

### Phase 2: Backend Deployment
1. Connect Railway to GitHub repo
2. Configure backend service:
   - Root directory: `/`
   - Dockerfile: `backend/Dockerfile`
   - Environment variables: DATABASE_URL, JWT_SECRET, ANTHROPIC_API_KEY, QDRANT_URL, QDRANT_API_KEY
3. Deploy and verify health endpoint

### Phase 3: Frontend Deployment
1. Configure frontend service:
   - Root directory: `/`
   - Dockerfile: `frontend/Dockerfile`
   - Port: 80
2. Deploy and verify static assets serve

### Phase 4: Environment Configuration
1. Set all required environment variables in Railway dashboard
2. Configure custom domain (if desired)
3. Verify WebSocket connections work through Railway's proxy
4. Test end-to-end functionality

---

## Open Questions

1. **Expected traffic**: How many concurrent users? (affects sizing decisions)
2. **Geographic distribution**: Where are users located? (US-only vs global)
3. **Budget constraints**: What's the maximum monthly spend acceptable?
4. **Deployment frequency**: How often will we deploy updates?
5. **Custom domain**: Do we need a custom domain, or is Railway's subdomain OK?

---

## Next Steps

1. **User Decision**: Select platform (Railway recommended)
2. **Create Railway account** (or selected platform)
3. **Create deployment plan document** with specific Railway configuration
4. **Execute migration** following phased approach above
5. **Document production environment variables** in .env.example
6. **Update README.md** with deployment instructions

---

## Additional Considerations

### Multi-Instance Scaling
- All three platforms support horizontal scaling
- Railway: Add more instances via dashboard, usage-based pricing scales
- Render: Configure instance count in Blueprint or dashboard
- Fly.io: Scale via fly scale command

### Current Docker Setup Compatibility
- **Render**: Highest compatibility (Blueprints = docker-compose.yml)
- **Railway**: Good (can reference docker-compose.yml structure)
- **Fly.io**: Manual conversion (need fly.toml config)

### Database Backups
- **Render**: Automated daily backups on paid Postgres plans
- **Railway**: Automated backups included
- **Fly.io**: Automated backups included

### Monitoring
- **Render**: Built-in metrics and logs
- **Railway**: Built-in logs and metrics
- **Fly.io**: Built-in metrics, more advanced options

---

## Sources

- [Render Pricing](https://render.com/pricing)
- [Railway Pricing](https://railway.com/pricing)
- [Fly.io Pricing](https://fly.io/docs/about/pricing/)
- [Render vs Railway vs Fly.io Comparison (2025)](https://medium.com/ai-disruption/railway-vs-fly-io-vs-render-which-cloud-gives-you-the-best-roi-2e3305399e5b)
- [PaaS Container Deployment Comparison (2024)](https://alexfranz.com/posts/deploying-container-apps-2024/)
- [Railway Alternatives Analysis](https://northflank.com/blog/railway-alternatives)
- [Render Alternatives Analysis](https://northflank.com/blog/render-alternatives)
