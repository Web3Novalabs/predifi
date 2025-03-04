#!/bin/bash

# Install Scarb (Cairo Package Manager)
if ! command -v scarb &> /dev/null; then
  echo "Installing Scarb..."
  curl -L https://github.com/software-mansion/scarb/releases/download/v2.9.2/scarb-installer.sh | bash
else
  echo "Scarb is already installed."
fi

export PATH="$HOME/.scarb/bin:$PATH"
scarb --version

chmod +x contracts/scripts/install-scarb.sh
