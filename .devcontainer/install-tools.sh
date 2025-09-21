# @ryzen-xp

set -e

echo "🚀 Setting up development environment..."

# Install dependencies
apt-get update && apt-get install -y curl git build-essential

# Install asdf
git clone https://github.com/asdf-vm/asdf.git ~/.asdf --branch v0.14.0
echo '. "$HOME/.asdf/asdf.sh"' >> ~/.bashrc
echo '. "$HOME/.asdf/completions/asdf.bash"' >> ~/.bashrc
. ~/.asdf/asdf.sh

# Read versions from .tool-versions
SCARB_VERSION=$(grep "^scarb " .tool-versions | awk '{print $2}')
FOUNDRY_VERSION=$(grep "^starknet-foundry " .tool-versions | awk '{print $2}')

echo "📦 Installing Scarb $SCARB_VERSION and Starknet Foundry $FOUNDRY_VERSION..."

# Add and install plugins
asdf plugin add scarb || true
asdf install scarb "$SCARB_VERSION"
asdf global scarb "$SCARB_VERSION"

asdf plugin add starknet-foundry || true
asdf install starknet-foundry "$FOUNDRY_VERSION"
asdf global starknet-foundry "$FOUNDRY_VERSION"

echo "✅ Environment ready!"
