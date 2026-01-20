 # Contributing to predifi

Welcome to Pridifi! We're building a Decentralized Outcome Prediction Protocol on the Stellar network, and we're excited to have you contribute. This guide will help you understand what's expected and how to make meaningful contributions.

## 📋 Before You Start

### Application Process
When applying for an issue:
- **Specify your ETA** (Estimated Time of Arrival)
- **Maximum ETA: 48 hours**
- After 24 hours, a finished work or draft PR is expected, otherwise you will be unassigned
- If you're facing challenges, communicate early by creating a draft PR and tagging the maintainer in the PR (not in the issue)

### Project Structure
```
predifi/
├── README.md
├── contract/      # Smart contracts
└── frontend/      # User interface
```

## 🎨 Frontend Contributions

### Design Reference
- **Figma Design**: [predifi Figma](https://www.figma.com/design/QMi4SBZnJ7HkXJxrW8pcNC/PredFI?m=auto&t=XU1jVpvKbR89dOu6-6)

### Standards and Best Practices
- Follow professional frontend standards
- Write clean, optimized code
- **Component Size Limit**: Maximum 150 lines of code per component
- Keep components lean and focused on single responsibilities
- Ensure responsive design matches Figma specifications
- Implement proper error handling and loading states

### Code Quality Expectations
- Use consistent naming conventions
- Write self-documenting code
- Implement proper TypeScript types (if applicable)
- Follow the existing code structure and patterns
- Optimize performance and bundle size

## 🔗 Smart Contract Contributions

### Development Resources
- **Soroban Developer Guide**: [Getting Started with Soroban](https://developers.stellar.org/docs/build/smart-contracts/getting-started/hello-world)

### Standards and Best Practices
- Write clean, modular code
- Break down complex logic into smaller, reusable functions
- **Add helpful code comments** to explain what's happening
- Document complex algorithms or business logic
- Follow Soroban best practices and conventions
- Ensure code is well-structured and maintainable

### Code Quality Expectations
- Implement proper error handling
- Write comprehensive tests
- Use clear variable and function names
- Add NatSpec-style documentation for public functions
- Consider gas optimization where applicable

## 🚀 Contribution Workflow

### 1. Fork and Clone
```bash
git clone https://github.com/YOUR_USERNAME/predifi-stellr.git
cd predifi-stellr
```

### 2. Create a Branch
```bash
git checkout -b feature/your-feature-name
```

### 3. Make Your Changes
- Follow the standards outlined above
- Test your changes thoroughly
- Keep commits atomic and meaningful

### 4. Submit a Pull Request
- **PR Description must include**: `Closes #[issue_id]`
- Provide a clear description of changes
- Include screenshots/gifs for UI changes
- List any breaking changes or dependencies
- Tag the maintainer if you need early feedback

### Example Commit Messages
```bash
feat: add wallet connection component
fix: resolve token claim validation issue
docs: update contribution guidelines
refactor: modularize payment processing logic
```

## 📞 Communication

### Getting Help
If you encounter blockers or need clarification:
1. Create a **draft PR** with your current progress
2. Tag the maintainer in the PR description
3. Explain the specific issue you're facing
4. **Do not** request help in issue comments

### Response Times
- Maintainers will review draft PRs within 24 hours
- Final PRs will be reviewed as soon as possible
- Be responsive to feedback and requested changes

## 🎯 Our Mission

Help us push predifi on the Stellar network fast by:
- Writing clean, maintainable code
- Following best practices and standards
- Communicating effectively
- Delivering quality work on time
- Making the codebase better with each contribution

## 🌊 Let's Build Together

Every contribution matters. Whether you're fixing a bug, adding a feature, or improving documentation, you're helping build something meaningful on Predifi.

**Questions?** Reach out to the maintainers in your draft PRs.

Let's make some waves! 🌊
