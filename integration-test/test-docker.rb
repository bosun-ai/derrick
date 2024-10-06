#!/usr/bin/env ruby

require 'json'
require 'net/http'
require 'socket'

def request(method, path, body=nil)
  body_json = JSON.generate(body) if body
  uri = URI("http://localhost:50080#{path}")
  response = Net::HTTP.start(uri.host, uri.port) do |http|
    request = Net::HTTP.const_get(method.capitalize).new(uri)
    request.body = body_json
    request['Content-Type'] = 'application/json'
    http.request(request)
  end

  if response.code.to_i >= 400
    raise "Request failed with status #{response.code}: #{response.body}"
  end

  JSON.parse(response.body)
end

begin
  file_dir = File.dirname(__FILE__)

  cmd = "cargo run -- -p docker -s http -w #{file_dir}/test_config.json"
  
  # Run the command in a separate process
  pid = Process.spawn(cmd)
  puts "Running command: #{cmd} (PID: #{pid})"
  
  # Wait for the process to start listening on port 50080
  connected = false
  while !connected do
    begin
      TCPSocket.open('localhost', 50080) { connected = true }
    rescue Errno::ECONNREFUSED
      sleep 0.3
    end
  end
  
  puts "Running tests..."

  # Test the API
  response = request(:get, '/workspaces')
  raise "Expected empty workspaces, got #{response.inspect}" unless response["workspaces"] == []

  response = request(:post, '/workspaces')
  raise "Expected workspace ID, got #{response.inspect}" unless response['id']

  id = response['id']

  response = request(:get, '/workspaces')
  raise "Expected empty workspaces, got #{response.inspect}" unless response.dig("workspaces", 0, "id") == id

  response = request(:post, "/workspaces/#{id}/cmd_with_output", { 'cmd' => 'echo hello' })
  raise "Expected output, got #{response.inspect}" unless response == "hello\n"

  puts "\n\nAll tests passed!\n\n"
ensure
  # Kill the process
  Process.kill('TERM', pid)
end
