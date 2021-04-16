Feature: Public server apis
  Scenario: Connect to server, query and validate time
    Given I connect to server
    When I query time
    Then The server responds with time
    And The response has the correct format
