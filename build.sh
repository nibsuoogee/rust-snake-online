#!/bin/bash

# Build Yew frontend
echo "Building Yew frontend..."
cd frontend
wasm-pack build --target web --out-dir dist
cp static/* dist/
cd ..

# Move built frontend files to `backend/public` (if serving from Node)
mkdir -p backend/public
cp -r frontend/dist/* backend/public/

# Install backend dependencies and build backend
echo "Setting up Node backend..."
cd backend
npm install
cd ..

echo "Build completed!"