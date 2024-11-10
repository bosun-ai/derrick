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

def run_with_derrick(options)
  command = "cargo run -- #{options.join(' ')}"
  pid = Process.spawn(command)
  puts "Running command: #{command} (PID: #{pid})"

  connected = false
  while !connected do
    begin
      TCPSocket.open('localhost', 50080) { connected = true }
    rescue Errno::ECONNREFUSED
      sleep 0.3
    end
  end

  yield pid
ensure
  Process.kill('TERM', pid)
end

def run_tests(provisioner_mode:)
  file_dir = File.dirname(__FILE__)

  options = ["-p", provisioner_mode, "-s", "http", "-w", "#{file_dir}/test_config.json"]
  run_with_derrick(options) do |pid|
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

    response = request(:post, "/workspaces/#{id}/cmd_with_output", { 'cmd' => 'ls ./code/swiftide-ask' })
    raise "Expected output, got #{response.inspect}" unless response.include?("Cargo.toml")

    # Test that the command is a shell script that can run `cd`
    response = request(:post, "/workspaces/#{id}/cmd_with_output", { 'cmd' => 'cd ./code/swiftide-ask && ls' })
    raise "Expected output, got #{response.inspect}" unless response.include?("Cargo.toml")

    # Test that we can run multiline commands
    response = request(:post, "/workspaces/#{id}/cmd_with_output", { 'cmd' => "echo hello\necho world" })
    raise "Expected output, got #{response.inspect}" unless response.include?("hello\nworld\n")

    # Test that the setup script ran successfully
    response = request(:post, "/workspaces/#{id}/cmd_with_output", { 'cmd' => 'cat /tmp/hello.txt' })
    raise "Expected output, got #{response.inspect}" unless response.include?("Hello World")
  end
end

["docker"].each do |provisioner_mode|
  puts "Running tests in #{provisioner_mode} mode..."
  run_tests(provisioner_mode: provisioner_mode)
end

puts "\n\nAll tests passed!\n\n"