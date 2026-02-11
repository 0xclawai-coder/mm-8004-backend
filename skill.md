# Backend Skills & Guidelines

## Backend Overview
API and data services for Molt Marketplace (ERC-8004)

## Technology Stack
TBD - Will be determined based on project requirements
Possible options:
- Node.js (Express, Fastify, NestJS)
- Python (FastAPI, Django, Flask)
- Go (Gin, Echo, Chi)
- Java/Kotlin (Spring Boot)

## Development Guidelines

### API Design
- RESTful or GraphQL architecture
- Clear endpoint naming
- Consistent response formats
- Proper HTTP status codes
- API versioning strategy

### Code Organization
- Modular architecture
- Separation of concerns
- Repository pattern for data access
- Service layer for business logic

### Database
- Choose appropriate database (SQL/NoSQL)
- Migration management
- Connection pooling
- Query optimization

### Authentication & Authorization
- Secure authentication mechanism
- Token-based auth (JWT, sessions)
- Role-based access control
- API key management

### Error Handling
- Consistent error responses
- Proper logging
- Error monitoring
- Graceful degradation

### Testing
- Unit tests for business logic
- Integration tests for APIs
- Database tests
- Load testing for performance

### Performance
- Caching strategies
- Rate limiting
- Database indexing
- Query optimization

### Security
- Input validation
- SQL injection prevention
- XSS protection
- CORS configuration
- Environment variable management
